# Mapping group rotation browser checker repair — 2026-09-09

The checker initially could not reach its browser assertions because it ignored
`CHROME_PATH` and per-user browser locations and fell back to the unavailable
Playwright headless shell. The bounded repair adds explicit environment paths,
`LOCALAPPDATA` Chrome/Edge paths, then the existing system paths.

- Source before this checkpoint: `main` at `0934362b57d5227f72bd4ebd8d3ad6b6144b3586`.
- Product source and group-rotation assertions: unchanged.
- `node --check app/scripts/check-mapping-group-rotation-browser.mjs`: PASS
- `node app/scripts/check-mapping-group-rotation-browser.mjs`: PASS in the
  configured per-user Chrome.
- Evidence covers selected previews, relative yaw and position preservation,
  unselected-fixture isolation, cancel, and confirmed snapshot correctness.
- `git diff --check`: PASS

This is a checker-only repair and does not claim native, hardware, macOS,
venue or product-wide completion.
