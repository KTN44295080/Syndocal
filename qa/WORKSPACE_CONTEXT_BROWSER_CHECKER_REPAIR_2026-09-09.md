# Workspace context browser checker repair — 2026-09-09

## Finding

`node app/scripts/check-workspace-context-content-browser.mjs` initially
stopped before running its assertions because the checker only searched the
system Chrome/Edge locations and did not honor this PC's per-user Chrome
installation. Playwright then attempted its missing bundled headless shell.
This was a checker environment-resolution defect, not a product assertion
failure.

## Bounded repair

`app/scripts/check-workspace-context-content-browser.mjs` now resolves browser
executables in this order: `CHROME_PATH`, `EDGE_PATH`, the current user's
Chrome/Edge locations under `LOCALAPPDATA`, then the existing system paths.
The component fixture and all assertions are unchanged. Product source files
were not modified.

## Evidence

On `main` before this checkpoint, source was `f2d745e71464ce25ccf79293d95e807eb8699377`.

- `node --check app/scripts/check-workspace-context-content-browser.mjs`: PASS
- `node app/scripts/check-workspace-context-content-browser.mjs`: PASS
  - one JSX context getter construction
  - no duplicate side-effect owner
  - replacement cleanup
  - hidden background toggling
  - fallback rendering
  - final owner disposal
- `git diff --check`: PASS

This repairs only checker browser discovery. It does not claim native-window,
hardware, macOS, venue or product-wide completion.
