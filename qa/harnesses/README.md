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

  Current honest standings (2026-08-09):
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
  and FX family/recipe selection. Daslight same-task observations now cover 12
  of those 13: eleven are equal, Touch control creation is one click shorter in
  Syndocal, and bulk Scene Live reset has no one-action Daslight equivalent.

  `app/scripts/check-project-shortcuts.mjs` adds two executable project tasks.
  It calls the production Save/Open dispatcher and fixes `Ctrl+S` and `Ctrl+O`
  at one key gesture. Native `.sdc` write/read and maximized Daslight Save/reopen
  were observed separately, yielding parity for Save (one operation) and named
  reopen (three operations). Patch, static programming, and FX target
  application remain unmeasured.

## External Art-Net monitor

- `artnet-monitor.mjs` is a separate-process ArtDMX receiver and 512-channel
  dashboard for the product's formal external-visualizer path. It parses UDP
  6454 fail-closed, tracks sequence continuity per sender/universe, records
  payload transitions and writes machine-readable JSON evidence. On Windows
  it shares UDP 6454 with an Art-Net application, so same-host Daslight and
  Syndocal capture can be attempted without changing the firewall. Add
  `--capture-changes` when every retained transition needs its full DMX byte
  array; `--max-transitions` bounds the resulting evidence size. Capture the
  two products sequentially: on Windows, leaving Daslight bound to 6454 can
  consume Syndocal's loopback unicast instead of duplicating it to the monitor.

- `artnet-compare.mjs` compares the last captured frame for a selected universe
  byte-for-byte. It reports exact one-based channel differences and returns 0
  for equality, 1 for a valid difference, and 2 for malformed/missing evidence.
  Transition digests are diagnostic because independently started dynamic FX
  captures are not guaranteed to share a time origin.

- `daslight-dmx-levels-reference.ps1` supplies a no-Art-Net Daslight reference
  path. Capture the visible `DMX LEVELS` child window as a 730x601 PNG, transcribe
  its non-zero numeric values, and the script writes comparator-compatible JSON.
  It rejects a changed crop, missing/unexpected blue level bars, and values whose
  bar estimate is outside the configured tolerance. This intentionally reads
  the supported visible UI; it does not inspect Daslight process memory or its
  protected internal visualizer transport.

  ```powershell
  node qa/harnesses/artnet-monitor.mjs --self-test
  node qa/harnesses/artnet-monitor.mjs --duration-seconds 20 --capture-changes --max-transitions 1024 --evidence target/qa/artnet-acceptance.json
  pwsh qa/harnesses/daslight-dmx-levels-reference.ps1 -SelfTest
  pwsh qa/harnesses/daslight-dmx-levels-reference.ps1 -InputPng target/qa/daslight-dmx-levels.png -Assignments "82=255,83=130" -OutPath target/qa/daslight-scene.json
  node qa/harnesses/artnet-compare.mjs --self-test
  node qa/harnesses/artnet-compare.mjs --reference target/qa/daslight-scene.json --candidate target/qa/syndocal-scene.json --universe 0 --evidence target/qa/artnet-comparison.json
  ```

  Open `http://127.0.0.1:6455/` while it is running. Loopback proves the
  Syndocal software/process boundary; a receiving PC, Art-Net node or fixture
  rig remains separate hardware acceptance. See
  `qa/ARTNET_EXTERNAL_VISUALIZER_ACCEPTANCE.md`.

## Bench launchers

- `check-asio-build.ps1`, `ndi_studio_monitor_launcher.rs`,
  `ndi_test_patterns_launcher.rs` — external I/O bench helpers (see
  `qa/M4_IO_VALIDATION.md`).
