# Timeline persistence audit — 2026-09-08

## Scope

This checkpoint audits the existing Timeline persistence, project-history, and
save/reload paths on current `main`. It does not add product behavior. The
previous real-file-missing → UI Retry → recovery test is not rerun; its
recorded result and ownership remain unchanged.

- Main base before this checkpoint: `80da30b25f0eb654e2392a887b941b0975d56d0c`
- Product source changes: none
- Changed file: this QA record only
- Physical output, external clients, browser UI, Mac hardware, signing, and
  publication were not exercised

## Static contracts

The following existing checkers passed against the base above:

| Checker | Result |
| --- | --- |
| `check-project-autosave-coordinator.mjs` | PASS |
| `check-project-history-keyboard.mjs` | PASS |
| `check-project-history-preflight.mjs` | PASS |
| `check-project-open-bootstrap.mjs` | PASS |
| `check-project-publication-e4.mjs` | PASS |
| `check-project-recovery-e3.mjs` | PASS |
| `check-project-storage-helpers.mjs` | PASS |
| `check-timeline-transport-runtime.mjs` | PASS |
| `check-timeline-navigator-actions.mjs` | PASS |
| `check-timeline-advanced-authoring.mjs` | PASS after the line-ending-only checker repair in `80da30b` |

The advanced-authoring checker retains its existing assertions. The preceding
checker-repair record explains that the current CRLF source was normalized for
inspection; no product source or acceptance assertion was weakened.

## Rust persistence evidence

All commands ran with `vcvars64.bat -vcvars_ver=14.44` and the exact
BuildTools `14.44.35207` x64 linker first in `where.exe link.exe`.

| Focused filter | Result |
| --- | --- |
| `project_publication_` | 18 passed, 0 failed |
| `project_backup_` | 2 passed, 0 failed |
| `project_coordinator_identity_swap` | 1 passed, 0 failed |
| `project_history_generation_preflight` | 1 passed, 0 failed |
| `project_history_skips_noops` | 1 passed, 0 failed |
| `project_save_reload_uses_authored_video` | 1 passed, 0 failed |
| `project_load_recomputes_stale_conform_rate` | 1 passed, 0 failed |
| `timeline_scene_block_edits_survive_project_history_entries` | 1 passed, 0 failed |
| `timeline_snap_batch_is_preserved_as_one_project_history_entry` | 1 passed, 0 failed |

The two Timeline filters were rerun by their exact test names after an earlier
broader filter matched zero tests. The zero-test invocations are not counted
as evidence.

These checks cover publication/recovery and backup boundaries, identity-swap
fencing, history generation/no-op handling, authored-video save/reload,
stale-derived-value recomputation, scene-block history persistence, and
single-entry snap batching. They do not prove browser/native UI behavior,
external client recovery, physical output, or the product-wide completion
ledger.

## Boundary

This audit does not close `TIMELINE-PERSISTENCE-001` or any parent completion
ledger row solely from software evidence. Remaining ledger rows and external,
hardware, browser, Mac, signing, and release gates remain governed by their
respective acceptance records.

## Current-main revalidation — 2026-09-09

The same bounded audit was rerun against current `main` at
`92909028aaac3906d0320993c0a1579cf42e7883`. Product source files were not
changed by this checkpoint; the prior missing-file thumbnail recovery was not
rerun.

The ten existing static checkers listed above all passed. The Rust filters were
run with `vcvars64.bat -vcvars_ver=14.44`, and
`where.exe link.exe` first resolved to
`C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`:

| Focused filter | Result |
| --- | --- |
| `project_publication_` | 18 passed, 0 failed |
| `project_backup_` | 2 passed, 0 failed |
| `project_coordinator_identity_swap` | 1 passed, 0 failed |
| `project_history_generation_preflight` | 1 passed, 0 failed |
| `project_history_skips_noops` | 1 passed, 0 failed |
| `project_save_reload_uses_authored_video` | 1 passed, 0 failed |
| `project_load_recomputes_stale_conform_rate` | 1 passed, 0 failed |
| `timeline_scene_block_edits_survive_project_history_entries` | 1 passed, 0 failed |
| `timeline_snap_batch_is_preserved_as_one_project_history_entry` | 1 passed, 0 failed |

This current-main revalidation strengthens the software evidence only. It does
not close the ledger row or claim browser/native UI, external-client,
physical-output, Mac, signing, or publication acceptance.

## Current-main persistence revalidation — 2026-09-12

The bounded Timeline/project persistence slice was rerun against current
`main` at source HEAD `9ada31b1d762ff65e4b94bd3d7c756056ceb59b3`;
`origin/main` matched before the run. No persistence schema or project
authority behavior was changed. The documented MSVC 14.44.35207 x64 linker
was pinned and returned first by `where.exe link.exe`.

| Check | Result |
| --- | --- |
| `node app/scripts/check-project-autosave-coordinator.mjs` | PASS |
| `node app/scripts/check-project-history-keyboard.mjs` | PASS |
| `node app/scripts/check-project-history-preflight.mjs` | PASS |
| `node app/scripts/check-project-open-bootstrap.mjs` | PASS |
| `node app/scripts/check-project-publication-e4.mjs` | PASS |
| `node app/scripts/check-project-recovery-e3.mjs` | PASS |
| `node app/scripts/check-project-storage-helpers.mjs` | PASS |
| `node app/scripts/check-timeline-transport-runtime.mjs` | PASS |
| `node app/scripts/check-timeline-navigator-actions.mjs` | PASS |
| `node app/scripts/check-timeline-advanced-authoring.mjs` | PASS |
| `cargo test ... project_publication_` | PASS — 18 passed, 0 failed |
| `cargo test ... project_backup_` | PASS — 2 passed, 0 failed |
| `cargo test ... project_coordinator_identity_swap` | PASS — 1 passed, 0 failed |
| `cargo test ... project_history_generation_preflight` | PASS — 1 passed, 0 failed |
| `cargo test ... project_history_skips_noops` | PASS — 1 passed, 0 failed |
| `cargo test ... project_save_reload_uses_authored_video` | PASS — 1 passed, 0 failed |
| `cargo test ... project_load_recomputes_stale_conform_rate` | PASS — 1 passed, 0 failed |
| `cargo test ... timeline_scene_block_edits_survive_project_history_entries` | PASS — 1 passed, 0 failed |
| `cargo test ... timeline_snap_batch_is_preserved_as_one_project_history_entry` | PASS — 1 passed, 0 failed |

The Rust total is 27 passed with no failures or ignored cases in these
focused filters. The checks retain exact publication/recovery and backup
boundaries, identity/generation fencing, no-op/coalescing behavior,
authored-video save/reload, stale-derived recomputation, and Timeline history
batching. `TIMELINE-PERSISTENCE-001` remains Open for browser/native UI,
external-client recovery, physical output, Mac, signing, publication, and
product-wide acceptance.
