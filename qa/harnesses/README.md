# QA Harnesses

Operational tools for verifying Syndocal beyond the automated test suites.
Windows-first (this project's primary bench), portable where noted.

## Desktop capture (native window evidence)

- `capture-window.ps1` — DPI-aware screenshot of a native window by title.
- `click-window-point.ps1` — DPI-aware click at window-relative coordinates.
  Move target windows to the primary monitor first; secondary monitors with
  different DPI scaling produce offset coordinates.

## Long-run supervision (born from the 341-minute silent hang, 2026-07-16)

- `run-matrix-watched.sh` — the ONLY sanctioned way to run the full viewport
  matrix unattended: live log file + stall watchdog that reports (and keeps
  the process for autopsy) instead of waiting forever. The matrix harness
  itself also fail-fasts on occupied ports 5173/9227 and recycles its
  browser between fixture phases; see `app/scripts/check-viewport-containment.mjs`.
- `codex-job-watch.mjs` — watches a Codex companion job; prints one line on
  terminal status or N minutes of log silence (default 5). `queued` and
  `starting` are treated as non-terminal, but silence still trips the stall
  branch (a job once died silently while queued).
- `cdp-autopsy.mjs` — inspects a possibly-frozen harness Chromium through the
  browser-level DevTools WebSocket (page-level WS connections are exclusive
  and would kill the harness's client). Distinguishes harness-side waits,
  native-dialog blocks, and hard renderer freezes. Pair with a CPU-delta
  sample: high CPU = layout/JS storm; near-zero CPU = lock/IPC wait.

Known freeze class (Chrome 150, headless=new): a long-lived session freezes
its renderer main thread after dozens of heavy fixture navigations — isolated
fresh-browser runs of the same scenarios pass. The matrix harness's
between-phase browser recycling (fresh profile each time) isolates it.

## Operation-count contract (goal evidence)

- `check-operation-counts.mjs` — performs core desk tasks with EXACTLY their
  budgeted number of user gestures via CDP and asserts the outcome. This is
  the executable half of the project rule "no operability-superiority claims
  without same-task operation-count measurements":
  - the Syndocal side is measured by this script (a UI change that adds a
    required gesture fails the task — operation-count regressions become
    visible instead of anecdotal);
  - the Daslight 5 side is recorded manually in
    `target/qa/ui-comparison/PRIMARY_OBSERVATIONS.md` from real sessions.
  - a missing Daslight observation means "no claim yet", never an assumed win.

  Current honest standings (2026-07-23, post-T18 rerun):
  | task | Syndocal | Daslight 5 |
  |---|---|---|
  | place scene on a timeline lane | 1 drag (always-visible Scene Matrix) | 1 drag (always-visible pool) |
  | toggle a layer's mute | 1 click | 1 click |
  | expand + restore timeline pane | 1 click + Esc | 2 clicks |

  T14/T15 closed the measured place-scene gap by keeping Scene Matrix and
  Timeline visible together. The harness now performs the production path as
  one direct drag and fails unless one Scene Block is added. T18 expands the
  executable Syndocal side to 13 tasks: the three rows above plus matrix
  trigger, Scene Live speed/reset, Touch open/trigger/flash/Edit/control-add,
  and FX family/recipe selection. The ten added tasks carry Daslight
  `未計測`; only the three rows above support a same-task parity statement.
  Patch, static programming, FX target application, and native Save/reopen
  are still outside this harness and remain unmeasured.

## Bench launchers

- `check-asio-build.ps1`, `ndi_studio_monitor_launcher.rs`,
  `ndi_test_patterns_launcher.rs` — external I/O bench helpers (see
  `qa/M4_IO_VALIDATION.md`).
