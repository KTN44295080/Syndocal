# Timeline shelf and checker revalidation — 2026-09-10

## Checkpoint

- Base `main`: `63ba6fd26bf4748f68aa82d21bf1a20a6f58d8ff`
- Working branch: `codex/timeline-shelf-compact-20260910`
- Scope: repair the existing viewport checker to match the current Timeline
  source-shelf contract and keep the shelf compact at the supported desktop
  viewports.
- Owner files: `app/src/styles.css` and
  `app/scripts/check-viewport-containment.mjs`.

## Changes

- The Timeline Sources shelf now reserves a 200px compact rail, shrinking only
  when the lower context pane is smaller; card overflow remains owned by the
  shelf body and the outer shelf remains non-scrolling.
- The checker now drives the current `data-timeline-source-shelf-filter="video"`
  filter instead of the retired two-tab category selector.
- The checker now finds the Overview frame through the current Navigator
  wrapper and verifies End-key traversal to the third `Video Preview` context
  mode.
- No Timeline source, IPC, mutation, or physical-output behavior was added.

## Evidence

- `pnpm.cmd --dir app build` — PASS (TypeScript and Vite production build).
- `node app/scripts/check-timeline-source-shelf-contract.mjs` — PASS.
- `pnpm.cmd --dir app run check:timeline-source-shelf` — PASS at 1920x1080;
  Scene click/drag and Media click/drag passed, media asset `900`, Video/Audio
  targets `15/11`, placement parity passed.
- `node app/scripts/check-timeline-performance-browser.mjs` — PASS at
  1920x1080, 1366x768, 860x520, and 1280x720; wheel contract and fixture
  geometry passed.
- `node app/scripts/check-timeline-authority-browser.mjs` — PASS at 1280x720;
  trusted-input rearm, exact payloads, invalid-target silence, and unknown
  command rejection passed. This is browser IPC contract evidence only.
- `pnpm.cmd --dir app run check:timeline-slim` — PASS at 1920x1080,
  1920x1032, 2048x1152, 1366x768, and 1280x720.
- `node app/scripts/run-control-upper-workspaces-browser.mjs` — PASS at
  3840x2160, 2560x1440, 1920x1080, and 1280x720 with zero CDP errors and
  owned browser/Vite/profile cleanup. The 1280x720 Timeline shelf measured
  200px and remained compact within the lower context pane.
- Related static contracts (external DnD, Timeline automation, Follow, cue
  audio, audio output bus, and loop runtime) — PASS.

## Boundaries and remaining work

- The checker repair does not prove native Tauri, physical output, ASIO/NDI,
  Mac hardware, signing, publication, or release acceptance.
- The user-directed file move operation for the real missing-file thumbnail
  recovery test was not performed; `missing -> Retry -> recovery` remains
  unclaimed.
- This checkpoint does not claim product-wide completion or venue acceptance.
