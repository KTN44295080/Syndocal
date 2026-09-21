# UI layout audit checkpoint — 2026-09-21

## Scope and state

- Branch: `codex/showclock-review-20260912`
- Base before this checkpoint: `21da77eb91bd37da9826b8b0c1c1ca8d6f003177`
- Audited the rendered Setup, Edit, and Control workspaces for visibly crushed,
  clipped, or unusable layout. This is a focused UI-layout checkpoint, not a
  closure of a broader Flow marker or product acceptance gate.
- Native visual inspection covered Setup Lighting/Patch, Video/Outputs, I/O,
  and Security/AI Access; Edit Lighting, Video, Both, and Timeline; and Control
  Lighting, Video, and Both. Navigation and observation only; no output,
  recording, Take, blackout, Arm, or device action was triggered.

## Finding and fix

The full Video mixer’s lower context area stacked Outputs and Layers vertically.
That split the available height between two independently scrolling panes and
left their content visibly compressed, especially with the Clip and Preview /
Program areas above them. The user-provided screenshot showed this same
crowding.

`app/src/styles.css` now keeps Outputs and Layers side-by-side in the shared
context row at all heights. The available context height is no longer divided
into two stacked halves. Typography, controls, and hit targets were not reduced.
`app/scripts/check-viewport-containment.mjs` now asserts that both panes stay
inside the context row, share its full height, and retain at least 250 CSS px
of width.

No other crushed/clipped layout was found in the inspected workspace screens.

## Verification

- `pnpm.cmd --dir app run check:edit-ia-video` — passed at `1920x1080`,
  `1920x1032`, `2048x1152`, `1366x768`, and `1280x720`; runtime exceptions
  absent and the new full-row containment assertion passed.
- `pnpm.cmd --dir app run check:topbar-pulse` — passed at `1280x720`; header,
  Pulse control, and group alignment remained within the viewport.
- The broad `check:viewport` harness exercised Setup, Edit, and Control screen
  assertions, including the available sequences at `1920x1080`, `1920x1032`,
  and `2048x1152`, but did not complete cleanly. At later navigation/reload
  transitions it repeatedly failed with CDP `Page.navigate did not reply
  within 15000 ms` (including isolated compact-viewport retries). This is an
  unresolved harness/navigation timeout, not a passing full-suite result; no
  preceding UI assertion failure was reported.
- `pnpm.cmd --dir app run check:tauri-build-wrapper` — passed (243 assertions,
  27 hostile mutation fixtures). The Windows build used the pinned Build Tools
  linker `14.44.35207`.
- `pnpm.cmd --dir app tauri build --no-bundle` — passed; generated
  `target/release/syndocal.exe`.
- Post-build exact-path launch — exactly one matching process, PID `39020`,
  title `Syndocal`, non-zero main-window handle, and `Responding=True`. The
  maximized native window was visually inspected at a 1275x800 logical capture.
  Executable SHA-256:
  `5017D0FA11A3B8BE2D7C51C7FBEFB414B4390B575E7A465E493A9AC7197128C1`.
  Edit Video showed Outputs and Layers side-by-side within the same full-height
  context row; labels and empty-state text remained visible without overlap.
  Control Video and the Timeline workspace were also opened in the rebuilt
  native app and remained responsive. No physical-output or risky show action
  was triggered.

## Evidence boundary

Rendered-browser checks establish the tested viewport contracts, not all
native DPI/scaling/accessibility combinations or physical display/output
behavior. The broad viewport harness timeout remains explicit. This focused
layout correction does not close UI-H5, the 58-marker completion ledger, or any
hardware, live-output, recording, failure-recovery, accessibility, or release
acceptance gate.
