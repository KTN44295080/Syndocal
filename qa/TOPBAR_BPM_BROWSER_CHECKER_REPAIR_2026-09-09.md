# Topbar BPM browser checker repair — 2026-09-09

## Findings

The checker had two stale assumptions before reaching the component contract:

1. It ignored `CHROME_PATH` and per-user browser locations, then fell back to
   the unavailable Playwright headless shell.
2. Its fixture searched for the Japanese accessible name `BPMを編集`, while
   the current `TopbarBpmControl.tsx` exposes `Edit BPM` as both aria-label and
   title.

Neither failure was a product behavior failure. The product component and its
normal/abnormal assertions were left unchanged.

## Bounded repair and evidence

- Source before this checkpoint: `main` at `710e2f3b3ce13705f3f78f454f3a68e3c7b85ee4`.
- `app/scripts/check-topbar-bpm-browser.mjs` now honors `CHROME_PATH`,
  `EDGE_PATH`, `LOCALAPPDATA` browser locations, then the existing system
  paths.
- The fixture locator now uses the component's existing `Edit BPM` accessible
  name.
- `node --check app/scripts/check-topbar-bpm-browser.mjs`: PASS
- `node app/scripts/check-topbar-bpm-browser.mjs`: PASS in the configured
  per-user Chrome, covering click/double-click/label editing, focus/no drag,
  Enter/blur/Escape, bounds, preserved draft, rejection, one pending commit
  and unmount safety.
- `git diff --check`: PASS

This is a checker-only repair. It does not claim native-window, hardware,
macOS, venue or product-wide completion.
