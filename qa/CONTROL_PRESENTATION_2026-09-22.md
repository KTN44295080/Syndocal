# Preview Transport and Control presentation — 2026-09-22

Branch: `codex/showclock-review-20260912`; base: `5528c4fa`.

## User-visible changes

The supplied Preview Transport image showed a nearly zero-width position slider,
an ellipsized time readout and buttons stretched through the monitor height.
The transport now uses its own container width: narrow rails place time/seek
on a full-width row and reflow the controls, preserving 44px button targets and
the existing 54px speed readout. Buttons retain 44px height. Exceptionally narrow
rails wrap Clear to a third row; overflow remains internal.

Operator Control (`touch`, not Edit) now uses a restrained dark surface, quieter
tile borders, consistent rounded controls and amber selected domain/mode states.
Lighting/Video/Both stay left, and the upper/lower-two shell remains intact.
Page add/delete appear in EDIT only; LIVE retains page selection. Saved tile
geometry, bindings, labels, sizing and data are unchanged. Domain styling is
isolated in `app/src/components/ControlWorkspace.css`.

The ImageGen concept was used for palette/hierarchy direction, not as a literal
functional specification: its invented fixtures, added buttons, clock and logo
were not implemented. Visual inspection compared palette, selection emphasis,
border density, pane structure and typography. Existing operator copy is unchanged.
This is a bounded presentation iteration, not a claim of pixel-identical concept
implementation or final operator design approval.

## Evidence

- `check-control-upper-workspaces-browser.mjs`: all nine supported viewport
  classes passed (3840x2160, 2560x1440, 2560x1504, 1920x1080, 1280x720,
  1280x752 DPR1, 1280x752 DPR2, 1280x800 DPR2, 1280x776 DPR2).
  The original combined run timed out navigating after the first viewport;
  remaining viewports were executed in fresh browser processes using the
  existing `SYNDOCAL_CONTROL_VIEWPORT` selector. The timeout is not counted as
  a full-suite pass. Zero runtime/console errors were reported in passing runs.
- New transport assertions cover readable full time, >=120px seek width,
  44–48px button heights and containment. The tight 1280x720 run also passed
  the added horizontal-containment assertion.
- Existing `check-viewport-containment.mjs --touch-only`: default and composed
  Control surfaces passed five sizes, including live control interactions and
  EDIT/LIVE return. Updated the old requirement for page-add/delete visibility
  in LIVE to hidden, and strengthened EDIT reachability to >=44px geometry.
- Focused domain sequence: Lighting -> Video -> Both -> Lighting in five sizes
  from 860x520 through 1920x1080, stable lower band, upper-only contents,
  unobstructed toolbar/domain controls, internal clip-pad scroll reachability,
  page management visible only in EDIT, no outer scroll or runtime errors.
- Inspected Chromium screenshots of Control Lighting and Preview Transport at
  1280x720 and 1280x752. Evidence is outside the repo in
  `%TEMP%/syndocal-control-layout-20260922` and
  `C:/TEMP/syndocal-control-ui-checkpoints`.
- Browser plugin unavailable; used existing direct Chromium CDP harnesses.
  No Computer Use, physical device action or subagent was used for this tranche.

## Boundaries

Final `pnpm.cmd --dir app tauri build --no-bundle` exited 0 with pinned VS2022
Build Tools MSVC 14.44.35207 first on PATH. Release compile took 3m03s.
First-party compiler warnings: 0 in this run; the existing Vite >500kB chunk
advisory remains (one advisory, unchanged policy; no threshold suppression).
The earlier build in this tranche also had 0 Rust warnings and the same advisory.

Exact release SHA256:
`1506F5CA847486ADEDF01B96338A48C92030A919F078532008EFE6B74958C124`.
Launched as PID 65048; one exact-path process, responsive Syndocal window,
maximized via the existing Tauri window API. Read-only native CDP capture at
1280x752 CSS / 2560x1504 physical shows Control Lighting, the left switch,
LIVE without page-edit buttons, the revised surfaces and the intact lower band.
Screenshot: `%TEMP%/syndocal-control-restyle-native.png`. No DOM navigation or
show/device commands were used. Native Preview Transport visual acceptance is
not claimed; that layout has the nine-class browser evidence above.

This artifact contains the pre-existing uncommitted frontend changes already
in the working tree; it is not a clean release provenance claim.

No hardware, clean-install, public-network, output or release acceptance is
claimed. UI-H5 and the overall closure goal remain open. The in-progress native
MCP runner belongs to the separate closure work recorded in
`GOAL_LEDGER_CLOSURE_2026-09-22.md` and is not part of this UI commit.
