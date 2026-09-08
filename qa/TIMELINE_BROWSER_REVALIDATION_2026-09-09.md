# Timeline browser revalidation and checker repairs — 2026-09-09

This checkpoint repairs two existing browser-harness assumptions and reruns
the affected Timeline reachability/geometry gates. Product source was not
changed.

- Source before this checkpoint: `main` at `6a838b8e1de1bafe29dcaa5735f47bf8958e6d26`.
- The completed real-file missing → UI Retry → restore → recovery result was
  not rerun.

## Checker repairs

- `check-timeline-performance-browser.mjs` now finds the existing
  `.timelineOverviewFrame` below the `timelineShowSurface` boundary instead of
  requiring a brittle direct-child relationship. The geometry assertions and
  duplicate/legacy chrome checks remain unchanged.
- `check-timeline-navigator-browser.mjs` now accepts the explicitly selected
  `CHROME_PATH` before its existing system Chrome/Edge candidates. It does not
  download or assume an unavailable Playwright browser shell.

## Verification

```text
CHROME_PATH=C:\Users\janua\AppData\Local\Google\Chrome\Application\chrome.exe node app/scripts/check-timeline-performance-browser.mjs
```

PASS at 1920×1080, 1366×768, 860×520, and 1280×720. The run reported the
Timeline wheel contract, phase/bank/media/selection state, linked
Scene/Audio/Video/Lighting automation and Video automation, unlinked local
routes, direct resize, and Alt-isolate behavior. Its own proof boundary is
browser fixture request/DOM/geometry only.

```text
CHROME_PATH=C:\Users\janua\AppData\Local\Google\Chrome\Application\chrome.exe node app/scripts/check-timeline-navigator-browser.mjs
```

PASS at 1280/640. Root/child selection, pending exclusion, internal scrolling,
focus/identity preservation across updates, collapse, and pending-unmount
cleanup passed.

- `node --check app/scripts/check-timeline-performance-browser.mjs`: PASS.
- `node --check app/scripts/check-timeline-navigator-browser.mjs`: PASS.
- `git diff --check`: PASS before commit.

No native application, physical output, device, external client, or Mac
environment was started. Browser evidence does not close native, hardware,
physical-output, persistence, Mac, signing, or product-wide completion gates.
