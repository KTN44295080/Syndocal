# Fixture group rotation history

Base `c212f08f7f53fc529946b084e96336c77bd2e70a`, branch
`codex/syndocal-v1.2`, product alpha.69 unchanged.

The preceding group-rotation change persisted each fixture separately. A single
gesture therefore created several history entries, and Ctrl+Z restored only one
fixture. Stage-click rotation and yaw-handle rotation now submit the entire group
through one renderer-ticketed `set_fixture_transforms` command. The single-fixture
command remains for its existing callers. Other layout/move operations are not
silently migrated by this change.

The frontend group commit has a small injected commit port and checks the whole
group against one refreshed authoritative snapshot. The command uses the existing
project transaction lifecycle, recovery receipt and operator/owner admission.
Captured project epoch and cancellation predicates are checked by the existing
facade before Begin and after Begin before mutation dispatch. An already admitted
native mutation may finish after the UI cancels; cancellation suppresses stale
UI results and does not claim to roll back a dispatched command.

## Evidence

- Controller regression script passes with one group dispatch for two fixtures,
  preserved positions/pitch/roll/relative yaw, pending preview, duplicate events,
  rejected commit, newer interactions and project replacement.
- TypeScript no-emit check passes.
- D2 receipt/recovery contract passes with the new tenth Stage route.
- Frontend Tauri inventory passes with 455 exact commands. The group commit port
  binds one literal command in App rather than exporting the generic invoke port.
- Independent frontend review accepted the transaction and stale-result wiring.

The broad `check-project-transaction.mjs` stops at its pre-existing line-25
raw-port regex. That regex also fails against `git show HEAD:app/src/App.tsx`
at the base above. This failure is not recorded as a pass and was not weakened.
Focused group/backend admission and history evidence must cover this change.
The subsequent [mutation lifecycle extraction](PROJECT_MUTATION_LIFECYCLE_2026-09-06.md)
corrects this exact stale composition assertion and passes the full checker.

## Backend evidence

Pinned Windows MSVC tests under `target/qa/mapping-group-undo-20260906/`:

- `engine-accepted.log`: 5 passed, 1066 filtered. Checks complete application,
  missing-last/invalid rejection without partial changes, the 4096-item bound,
  identical reapplication and rollback after publication failure. Fixture lookup
  builds one ID index: O(total fixtures + group size), with no per-frame work.
- `native-tests-final.log`: 3 passed, 1776 filtered. The actual project transaction
  Begin/Commit and history navigation restore both fixture transforms with one
  Undo and Redo. An invalid final fixture preserves both the image and history.
  Output retirement/persistence platform ports are test doubles; this does not
  claim physical output or GUI key handling acceptance.
- `admission-tests.log`: 5 passed, 1774 filtered, including the plural route's
  exact nested envelope and flat/mixed/non-object rejection.
- `operator-contract-accepted.log`: 515 registered commands, 133 transactional
  mutations; the frozen admission hash matches the generated handler inventory.
- Independent backend review accepted full-vector validation before mutation,
  one Stage publication, the existing digest/receipt recovery path, and history
  integration. Group-specific reply-loss execution was not separately repeated.

Two new-test unused-binding warnings were removed; final compiler warnings are
0 against previous accepted 0 (delta 0). The first history run revealed an invalid
custom-profile test seed; it was corrected to use the existing D4 fixture profile
convention. The Unchanged test compares persistence and published snapshots each
against their own prior projection, since published runtime layers differ from
persistence layers. Neither failure was hidden by weakening product assertions.

Native release/startup gate passed on 2026-09-06 after the user requested continuation.
The application was already stopped before the build; no live process was terminated.
Pinned MSVC release build passed in 2m 27s (Vite 7.71s), compiler warnings 0
(previous accepted 0, delta 0). Exact executable SHA256:
`42F17ABDCE1CBE8EB097644DD5D7509E4F457B0F766DB9211E0277D733B61D14`.
Launch evidence confirms one responsive maximized Syndocal main window, PID 98368.
The existing working copy DSF2026-before-output-read.sdc was opened through the
official single-instance path. Read-only MCP confirmed 46 fixtures, two video
outputs and healthy runtime (0 mutations). Physical output was not armed.
Logs: `target/qa/mapping-group-undo-20260906/native-build.log`,
`native-launch.json`, `native-readiness.json`. Native group rotation keyboard
acceptance remains unobserved; test a new rotation followed by Undo and Redo.
