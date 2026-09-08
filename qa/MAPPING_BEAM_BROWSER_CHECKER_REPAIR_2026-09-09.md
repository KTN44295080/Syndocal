# Mapping beam browser checker repair — 2026-09-09

The checker initially stopped before its Mapping assertions because it ignored
`CHROME_PATH` and per-user browser locations and fell back to the unavailable
Playwright headless shell. The bounded repair adds explicit environment paths,
`LOCALAPPDATA` Chrome/Edge paths, then the existing system paths.

- Source before this checkpoint: `main` at `dc702722e4ece51c6e8a63f7ada8df0878d6f673`.
- Product source and Mapping assertions: unchanged.
- `node --check app/scripts/check-mapping-beam-browser.mjs`: PASS
- `node app/scripts/check-mapping-beam-browser.mjs`: PASS in the configured
  per-user Chrome.
- Evidence covers profile optics, three emitted beams, reactive mounting tilt
  and vertical footprint.
- `git diff --check`: PASS

This is a checker-only repair and does not claim native, hardware, macOS,
venue or product-wide completion.
