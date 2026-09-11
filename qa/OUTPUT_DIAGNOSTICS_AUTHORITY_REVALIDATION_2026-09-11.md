# Output diagnostics project-authority revalidation — 2026-09-11

## Scope

This checkpoint repairs the renderer-side output diagnostics lifecycle so a
route action cannot publish a success/error message or continue a route
mutation after the project authority captured at action start has been
replaced. The existing backend output-control receipts, lease fencing,
Standby fail-closed behavior, and physical-output guards remain authoritative.

This is a bounded stale-result repair. It does not close the completion ledger,
AI3 durable external acceptance, physical DMX/Art-Net/Spout acceptance, Mac or
Linux acceptance, signing, publication, or the unfinished Channel API. The
previous real-file thumbnail missing → UI Retry → restore → recovery trial was
not rerun and is not claimed here.

## Owned changes

- `app/src/createOutputDiagnosticsController.ts` captures the current
  `ProjectAuthorityToken` for staged Art-Net, USB-DMX safety, Show DMX prepare,
  DSF2026 probe reconciliation, and Spout actions. Each await boundary checks
  exact token identity; stale work is discarded and stale errors are not shown.
- `app/src/App.tsx` supplies the existing project-authority capture/current
  predicates to the controller.
- `app/scripts/check-dmx-show-setup.mjs` adds a deterministic stale-during-
  lease-preparation regression. It proves no route action and no stale error or
  success message is published after authority replacement.
- `app/scripts/check-output-control-runtime.mjs` accepts the optional
  operation label used to keep Art-Net receipt-convergence diagnostics precise.

## Verification

Source base before this checkpoint: `31d015c375c512130c96e009fd0bd1816baf92c0`.

| Check | Result |
| --- | --- |
| `node app/scripts/check-dmx-show-setup.mjs` | PASS — canonical lease acquire/reuse, fail-closed preparation, singleflight/busy rejection, and stale-result suppression |
| `pnpm.cmd --dir app run check:output-control-runtime` | PASS |
| `pnpm.cmd --dir app run check:dvc-dmx-shortcuts` | PASS — 41 assertions |
| `pnpm.cmd --dir app run check:frontend-command-routing` | PASS — 133 renderer, 31 server-authoritative, 28 raw, 464 facade dispatches |
| `pnpm.cmd --dir app exec tsc --noEmit` | PASS |
| `pnpm.cmd --dir app run check:release` | PASS — ledger remains `50 Open + 8 Deferred` |
| MSVC-pinned `pnpm.cmd --dir app tauri build --no-bundle` | PASS — `link.exe` 14.44.35207 first in `where.exe` |
| Native launch/IPC probe | PASS — `target/qa/native-final-validation-20260911-67/native-final-validation.json` |
| `git diff --check` | PASS |

## Exact native artifact

- Path: `target/release/syndocal.exe`
- Version: `1.2.0-alpha.69`
- Size: `64,744,448` bytes
- SHA-256: `DBA10E32639E6B592F9FF35F25A59F00461E7EC6A396AF02A58FE04FE8C29B6E`
- Probe: one responsive maximized `Syndocal` window; Standby with lighting/video
  disabled; initial snapshot arrays present with zero video outputs; missing
  media and layer thumbnail IPC requests rejected with expected errors; zero
  physical-output operations; exact application exited; debug listener count
  returned to zero.

## Boundary

The repair protects the renderer from applying stale output-diagnostics results
after project replacement. It does not claim the external or physical rows in
Sections 6–9 are accepted, and it does not change their ledger status.

## Current-main output-local software rerun — 2026-09-12

The bounded output-local software gate was rerun against current `main` at
`2a770915974123c29433c6777f027d7482948743`, with the documented MSVC
14.44.35207 x64 absolute linker pin confirmed first by `where.exe link.exe`.

All focused checks passed: output ownership, output-control/Standby Sync,
safety blackout, video routing/window runtime/observation; engine ownership
(9/9); Tauri ownership (5/5), managed Spout terminal (2/2), Spout retirement
(5/5), callback-failure retirement (1/1), disabled activation compensation
(2/2), and managed-display projection (1/1). No physical output was enabled.
