# Mapping live movement browser checker repair — 2026-09-09

The checker initially stopped before its assertions because it ignored
`CHROME_PATH` and per-user browser locations and fell back to the unavailable
Playwright headless shell. The bounded repair adds explicit environment paths,
`LOCALAPPDATA` Chrome/Edge paths, then the existing system paths.

- Source before this checkpoint: `main` at `407113d326348a90b64eadfa71aac951b597c076`.
- Product source and live-movement assertions: unchanged.
- `node --check app/scripts/check-mapping-live-movement.mjs`: PASS
- `node app/scripts/check-mapping-live-movement.mjs`: PASS in the configured
  per-user Chrome.
- Evidence covers Pan/Tilt attribute motion, stale programmer isolation,
  16-bit DMX precedence, cache invalidation, and preview-removal fallback.
- `git diff --check`: PASS

This is a checker-only repair and does not claim native, hardware, macOS,
venue or product-wide completion.
