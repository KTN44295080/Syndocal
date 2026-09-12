# AI2 authored command bridge checkpoint — 2026-09-13

## Marker and boundary

- Marker: `AI2-COMMAND-BRIDGE-001`
- Branch: `codex/showclock-review-20260912`
- Base at implementation start: `8b628ade`
- Result: `Complete` for the supported local authored command bridge

This checkpoint closes the AI2 local authored-command contract only. AI4
consent/principals, AI5 sidecar adapters, AI6 administration UI, AI7 broad
adversarial parity, AI8 external/native acceptance, and physical output remain
separate markers and are not claimed here.

## Implemented contract

The server-authoritative Bank/Scene routes now use the same bounded terminal
receipt boundary as the existing authored effect route:

- `request_id` is required, ASCII, non-empty, and bounded.
- The receipt key binds operation, request, invoking WebView label, server
  owner identity, owner incarnation, and the starting E/R/H authority token.
- The wrapper binds the historical caller-supplied owner to the invoking
  WebView before receipt lookup and uses the server-derived binding for the
  mutation.
- Concurrent duplicates are single-flight. An exact retry returns the stored
  JSON result or terminal error without a second engine publication.
- Reusing a request identity with a different shape, or after owner retirement,
  fails closed. Retention and retired tombstones are bounded.
- Existing authoritative commit cores continue to publish one coordinator
  image and provide the existing exact Undo/Redo behavior; the receipt wrapper
  does not add a second history or publication path.

Covered local authored routes:

| Operation | Tauri route |
| --- | --- |
| `syndocal.effects.set_enabled.v1` | `set_effect_enabled` |
| `syndocal.cue_lists.reorder.v1` | `reorder_cue_lists` |
| `syndocal.cue_lists.rename.v1` | `rename_cue_list` |
| `syndocal.cue_lists.delete.v1` | `delete_cue_list` |
| `syndocal.scenes.create.v1` | `create_scene_authoritative_v1` |

The UI-only `create_cue_list` route is also protected by the generic receipt
lane, including the same owner retirement and exact retry behavior.

## Evidence

All checks below were run from the repository root after the implementation:

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:ai2-authored-command-bridge:self-test` | PASS — 2 self-test cases |
| `pnpm.cmd --dir app run check:ai2-authored-command-bridge` | PASS — canonical route inventory, request identity, E/R/H, owner retirement, and focused legacy gates |
| `pnpm.cmd --dir app run check:agent-bridge` | PASS — 11 groups |
| `pnpm.cmd --dir app run check:frontend-invokes` | PASS — 464 commands |
| `pnpm.cmd --dir app run build` | PASS — TypeScript and Vite production build |
| `cargo test -p protocol --release --locked control_plane_query -- --test-threads=1` | PASS — 15/15 |
| `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 authored_control_plane -- --test-threads=1` | PASS — 16/16, including generic exact retry/shape conflict/owner retirement |
| `pnpm.cmd --dir app tauri build --no-bundle` | PASS — fixed MSVC 14.44.35207; built `target/release/syndocal.exe` |
| exact executable process smoke | PASS — one exact-path process, `Responding=true`, window handle present; stopped after verification |

The native Rust tests used `vcvars64.bat -vcvars_ver=14.44`; the first
`where link.exe` result was
`C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`.

The Tauri build repeated the same linker pin and produced the release
executable. Native process smoke verified the exact checkout path only; no
button-level native UI or physical-device acceptance is claimed.

No physical device, external client, LAN, signed installer, or release-native
acceptance claim is made by this checkpoint.

## Changed files

- `app/src-tauri/src/authored_control_plane.rs`
- `app/src-tauri/src/main.rs`
- `app/src-tauri/src/scene_creation.rs`
- `app/src/App.tsx`
- `app/scripts/check-ai2-authored-command-bridge.mjs`
- `app/package.json`
- completion-flow and ledger expectation/check fixtures

Next action after this checkpoint is the next Open marker, `AI4-CONSENT-001`.
