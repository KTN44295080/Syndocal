# Timeline inline-child browser checker repair — 2026-09-09

The checker initially stopped before its component assertions because it did
not honor `CHROME_PATH` or per-user browser locations and fell back to the
missing Playwright headless shell. The repair adds the same bounded browser
resolution used by the other browser checkers: explicit environment paths,
`LOCALAPPDATA` Chrome/Edge, then the existing system paths.

- Source before this checkpoint: `main` at `f4b11805e262576ac1a49831c143e40698df186c`.
- Product source: unchanged.
- `node --check app/scripts/check-timeline-inline-child-browser.mjs`: PASS
- `node app/scripts/check-timeline-inline-child-browser.mjs`: PASS in the
  configured per-user Chrome.
- Evidence covers two viewports, shared cue fill/band/text, group-color
  fallback, reactive color updates without block replacement, overlap rails,
  viewport clipping, readonly pointer isolation, drill-in keyboard/double-click
  and collapse.
- `git diff --check`: PASS

This is a checker-only repair and does not claim native, hardware, macOS,
venue or product-wide completion.
