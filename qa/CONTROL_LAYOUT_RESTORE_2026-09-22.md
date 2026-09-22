# Control layout restoration — 2026-09-22

Branch: `codex/showclock-review-20260912`.
Base: `8c73e85b911a457bfd6993daabb2966d56991eea`.

## Change

Operator-facing Control is the internal `touch` route, not Edit (`control`).
Its Lighting / Video / Both switch is now left-aligned. Video stays within the
upper workspace row. Both divides only that upper row; it no longer creates an
implicit right column or spans across the lower Stage / Faders band.

Lighting shares the existing toolbar row, with space reserved for the domain
switch and the original desktop toolbar height preserved. Controls and tile
sizes are unchanged. Video content uses intrinsic rows and internal scrolling
instead of collapsing the clip bank when the upper pane is short.

Only `app/src/styles.css` and this note are owned by this checkpoint. Existing
App.tsx, localization, DJ Link checker, AI3 note, PIN helper and .vite changes
are unrelated and remain unstaged.

## Evidence

- Existing `node app/scripts/check-viewport-containment.mjs --touch-only`:
  default and composed Control surfaces pass at 1920x1080, 1920x1032,
  2048x1152, 1366x768 and 1280x720. Assertions were not weakened.
- Focused real Chromium fixture at `http://127.0.0.1:5189/?syndocalViewportFixture=vj-bank`:
  Lighting -> Video -> Both -> Lighting at 1920x1080, 1920x1032, 1366x768,
  1280x720 and 860x520. Passed left alignment, stable full-width lower band,
  two visible lower panes, upper-only panel placement, no outer scroll,
  unobstructed domain/toolbar hit targets and clip-pad scroll reachability.
  Title is Syndocal, meaningful content rendered, no Vite overlay or runtime
  exceptions/console errors. Reviewed Lighting/Both screenshots at 1280x720.
- Browser plugin not available; used the repository's existing Chromium CDP
  fixture approach. Temporary runner, screenshots and logs live outside the
  repository in `%TEMP%/syndocal-control-layout-20260922`.
- Independent final CSS review: no actionable findings.
- `git diff --check` passed.

## Boundaries

The broader `check-video-clip-slot-bank-browser.mjs` stops before reaching
Control at its Edit advanced-FX consumer count assertion (7 actual, 1 expected,
line 250). This CSS-only change does not alter Edit or mount component consumers;
the broad video gate is not claimed as passed.

This is local presentation work. No native rebuild/window acceptance or hardware
output was performed; compiler warnings were not measured. Next native release
verification should include the Control domain sequence against its built binary.
