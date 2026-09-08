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
