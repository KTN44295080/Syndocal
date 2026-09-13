# Timeline persistence checkpoint — 2026-09-13

This checkpoint closes `TIMELINE-PERSISTENCE-001` for the supported
current-source project-authority, Timeline editor/runtime, and rendered-browser
scope. The implementation was already present on the checkpoint base; this
unit records fresh evidence and updates the authoritative ledgers. No product
behavior or acceptance assertion was weakened.

## Implemented contract verified

- Project save/reload preserves authored media authority and recomputes stale
  derived values instead of trusting persisted runtime state.
- Undo/Redo uses generation and authority preflight, skips no-ops, coalesces
  continuous edits by target, and retains Timeline scene-block edits.
- Snap batches are represented as one project-history entry, while backup,
  publication, recovery, and coordinator identity swaps retain their fencing
  and ownership boundaries.
- The Timeline navigator keeps group selection/focus stable across reactive
  array updates, excludes pending child selection, supports root/child
  selection, internal scrolling, collapse, and pending-unmount cleanup.
- Guide/cue-audio routing, Timeline transport/advanced contracts, and the
  viewport containment matrix remain green on the current source.

## Verification

The browser plugin was unavailable in this environment. The documented regular
Playwright fallback used the installed Chrome executable for the navigator
fixture at `1280x600` and `640x600`, and the Timeline viewport matrix covered
the existing browser fixtures at `1920x1080`, `1366x768`, `860x520`, and
`1280x720` plus direct resize.

| Check | Result |
| --- | --- |
| `node app/scripts/check-project-autosave-coordinator.mjs` | PASS |
| `node app/scripts/check-project-history-keyboard.mjs` | PASS |
| `node app/scripts/check-project-history-preflight.mjs` | PASS |
| `node app/scripts/check-project-open-bootstrap.mjs` | PASS |
| `pnpm.cmd --dir app run check:project-publication-e4` | PASS |
| `pnpm.cmd --dir app run check:project-recovery-e3` | PASS |
| `pnpm.cmd --dir app run check:project-storage` | PASS |
| `node app/scripts/check-timeline-navigator-actions.mjs` | PASS |
| `pnpm.cmd --dir app run check:timeline-advanced` | PASS |
| `pnpm.cmd --dir app run check:timeline-viewport` | PASS — viewport matrix and overlap/scene-block fixtures |
| `PLAYWRIGHT_MODULE_PATH=...; CHROME_PATH=...; node app/scripts/check-timeline-navigator-browser.mjs` | PASS — 1280/640 containment, internal scrolling, root/child selection, pending exclusion, collapse, and cleanup |
| Tauri focused persistence filters | PASS — 27 passed, 0 failed, 0 ignored |

The Tauri filters were run by exact names: `project_publication_` (18),
`project_backup_` (2), `project_coordinator_identity_swap` (1),
`project_history_generation_preflight` (1), `project_history_skips_noops`
(1), `project_save_reload_uses_authored_video` (1),
`project_load_recomputes_stale_conform_rate` (1),
`timeline_scene_block_edits_survive_project_history_entries` (1), and
`timeline_snap_batch_is_preserved_as_one_project_history_entry` (1).

The Cargo checks ran after `vcvars64.bat -vcvars_ver=14.44`, with the required
BuildTools `14.44.35207` x64 linker first in `where.exe link.exe`:

```text
C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe
```

## Boundary

This closes the current-source project persistence, Timeline history/editor,
Guide routing, and rendered-browser viewport/navigator slice. It does not claim
native renderer/GPU/display behavior, real A/V/Lighting synchronization,
audible devices, external clocks, physical output, external-client recovery,
venue/soak operation, cross-platform execution, signing, publication, or
product-wide Timeline acceptance. Those boundaries remain represented by the
appropriate open or deferred markers.

`git diff --check`: PASS before commit.
