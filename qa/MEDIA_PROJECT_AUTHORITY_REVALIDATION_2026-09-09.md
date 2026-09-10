# Media / project authority revalidation — 2026-09-09

This is a focused revalidation checkpoint for the current `main` tree. It
does not add product behavior or reimplement the completed project-transaction
extraction. The purpose is to keep the existing media-authority and project
mutation boundaries evidenced while the broader handoff remains open.

## Source and scope

- Source: `main` at `f6a2b249c23268e754d112292286bc43b4fa4960`
- Remote comparison: `origin/main` matched before the checks.
- Product source changes: none.
- Native/hardware scope: none; these are deterministic source-level checks.

## Evidence

All commands were run from the repository root on 2026-09-09:

| Check | Result | Boundary covered |
| --- | --- | --- |
| `node app/scripts/check-media-asset-authority.mjs` | PASS | phase order, reply-loss/query, cancellation CAS, E/R/H continuity, operator classification, paired apply, empty-catalog normalization |
| `node app/scripts/check-project-transaction-mutation-controller.mjs` | PASS — 6 scenarios | one raw dispatch, ordinary failure, not-published recovery, published recovery, indeterminate/unconfirmed hold, post-Begin abort; no native/UI side effects |
| `node app/scripts/check-project-history-preflight.mjs` | PASS | production wrapper, Undo/Redo, cancel, busy, flush failure, stale/epoch fence, initial dirty mapping, current-entry CAS, redo invalidation |
| `node app/scripts/check-project-autosave-coordinator.mjs` | PASS | single-flight, failure cooldown, exact errors, success reset, main/pane ownership and App wiring |
| `pnpm.cmd --dir app run check:project-transaction` | PASS | executable project-transaction production contract and deterministic project authority |

No assertion was weakened and no external or physical-output acceptance is
claimed. The GUI cancellation/stop-latency, real-file media recovery,
snapshot full-show performance, macOS, hardware and venue boundaries remain
open under the handoff.

## Current-main revalidation — 2026-09-09

The same bounded checks were rerun against current `main` at
`e2cf572514b3f281dcdebb42aa6dc9a164ddc304`. Product source files were not
changed by this checkpoint, and the previously completed real-file thumbnail
recovery was not rerun.

The following all passed: `check-media-asset-authority.mjs`,
`check-project-transaction-mutation-controller.mjs` (6 scenarios),
`check-project-history-preflight.mjs`, `check-project-autosave-coordinator.mjs`,
and `pnpm.cmd --dir app run check:project-transaction` (production transaction
contract plus deterministic project authority). No assertion was weakened.

This is current-main software evidence only. It does not close
`MEDIA-DERIVED-001` and does not claim real-file recovery, GUI cancellation or
stop latency, cache/waveform/proxy performance, native/browser reachability,
external clients, physical output, Mac, signing, publication, or venue
acceptance.

## Media operation lifecycle responsibility checkpoint — 2026-09-10

The candidate was rechecked from `main` at `09b23af6e0db7eaae2afe78bb87affde0b1fa441`.
The existing media operation behavior was moved from `App.tsx` into
`app/src/mediaAssetOperationController.ts`; the App still owns the reactive
publication, while the focused controller owns operation IDs, AbortControllers,
phase updates, exact cancellation, release, and owner cleanup. No Channel API,
screen ticket, native command, or product behavior was added.

The following passed without changing assertions: `pnpm.cmd --dir app run
check:media-asset-operations` (phase publication, exact cancellation,
idempotent release, owner cleanup), `pnpm.cmd --dir app run
check:media-thumbnails`, `pnpm.cmd --dir app run check:project-transaction`,
`pnpm.cmd --dir app exec tsc --noEmit`, `pnpm.cmd --dir app run build`, and
`pnpm.cmd --dir app run check:release` (including the new controller check).
`git diff --check` also passed.

This remains a source-level frontend responsibility checkpoint. It does not
claim the unexecuted real-file missing-to-recovery GUI trial, native window/IPC
acceptance, physical output, macOS, signing, publication, or venue acceptance.
