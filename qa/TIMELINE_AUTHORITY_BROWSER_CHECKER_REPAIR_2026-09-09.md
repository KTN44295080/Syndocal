# Timeline authority browser checker repair — 2026-09-09

This checkpoint repairs the existing browser checker only. The product source
was not changed.

- Source/base before this checkpoint: `main` at `a88354c9acff25dbcdd3a35b653cb6d810086301`.
- Failure reproduced before the repair: after opening child Timeline `Shin`,
  the checker found Lighting lane options `51`/`52` in the DOM but rejected the
  target because the existing `Timeline click controls` disclosure was closed,
  so the select was not a visible hit-verified pointer target.
- Repair: the checker now opens the exact Lighting placement disclosure through
  the existing hit-verified pointer path before inspecting and keyboard-selecting
  the lane. The lane-option and pointer-hit assertions remain strict.
- `node --check app/scripts/check-timeline-authority-browser.mjs`: PASS.
- `CHROME_PATH=C:\Users\janua\AppData\Local\Google\Chrome\Application\chrome.exe`
  `node app/scripts/check-timeline-authority-browser.mjs`: PASS, 1280×720.
  The gate reported injected-fault latch, trusted-input-only rearm, exact
  registration/handoff/split/move payloads, invalid-target silence, and
  unknown-command rejection.
- Gate cleanup force-terminated only its owned headless-browser/Vite process
  trees; no product or physical output was enabled.
- Proof boundary: this is browser-side renderer IPC request-contract evidence.
  It does not prove native Tauri, backend coordinator, persistence, hardware,
  physical output, macOS, or product-wide completion.

`git diff --check`: PASS before commit.
