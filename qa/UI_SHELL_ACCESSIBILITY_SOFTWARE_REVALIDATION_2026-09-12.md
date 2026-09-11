# UI shell and accessibility software revalidation — 2026-09-12

This checkpoint records the bounded software/static evidence for the existing
UI shell and accessibility contracts. It does not add product behavior or
replace the native accessibility acceptance.

## Source and ownership

- Source base: `affb5290744ebf3d2b3d2b28eef1edf860dcba6e` (`main`)
- `origin/main` matched before the checks; the working tree was clean.
- No product source was changed for this checkpoint. Only this QA record and
  the two ledger mirrors are owned by the checkpoint.
- Browser viewport checks were not run here because the checker requires a
  locally installed Chrome/Edge executable, and neither supported executable
  exists on this PC. This is an environment boundary, not a product pass.

## Focused checks

The following 13 static/source checks were run against the exact source base:

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:frontend-command-routing` | PASS — 133 renderer, 31 server-authoritative, 28 raw, 464 facade dispatches |
| `pnpm.cmd --dir app run check:frontend-invokes` | PASS — 457 exact Tauri commands |
| `pnpm.cmd --dir app run check:desktop-window` | PASS — primary-only window mode and safe fullscreen shortcuts |
| `pnpm.cmd --dir app run check:localization` | PASS — 3,689/3,689 static Japanese UI coverage; 0 unprotected labels |
| `pnpm.cmd --dir app run check:terminology` | PASS |
| `pnpm.cmd --dir app run check:tauri-build-wrapper` | PASS — 243 assertions, 27 hostile mutation fixtures |
| `pnpm.cmd --dir app run check:empty-states` | PASS |
| `pnpm.cmd --dir app run check:project-shortcuts` | PASS — 15,552 matrix cases; App=30 and ProjectFile=3 reachable kinds |
| `pnpm.cmd --dir app run check:workspace-operator` | PASS — 28 assertions |
| `node app/scripts/check-pane-window-lifecycle.mjs` | PASS |
| `node app/scripts/check-project-history-keyboard.mjs` | PASS — Undo/Redo routing preserves text-editor native behavior |
| `node app/scripts/check-timeline-space-keyboard.mjs` | PASS — selected Timeline Space routing and guard cases |
| `pnpm.cmd --dir app run check:stage-labels` | PASS — anchoring, eligibility, priority, and overlap contracts |

## Boundary and non-claims

These checks provide bounded static/source evidence for shared routing,
discoverability-related contracts, keyboard paths, localization, labels,
window/pane ownership, and safety guards. They do not prove every supported
feature is reachable through H1, or that H2/H3/H4/H5 are complete in the
native application.

The following remain unmeasured and therefore remain open: native 2560x1440
and 1280x720 placement, the full three-screen operator state, NVDA or another
screen reader, High Contrast, color-independent state/error presentation,
125/150/200% display scaling, IME composition, dialog/popout focus return,
reduced-motion behavior, and native keyboard-only safety workflows.

`COV-UI-SHELL-001` and `COV-ACCESSIBILITY-001` remain `In progress`; this
checkpoint records software evidence without changing that status or claiming
native accessibility acceptance.

## Current-main UI shell rerun — 2026-09-12

The 13 UI-shell software/static checks were rerun against current `main` at
`0017c55b1f45ede38fed7e3de29f68fab37ee68d` and all passed:

- frontend command routing and invoke inventory — PASS;
- desktop-window, localization, terminology, build-wrapper, and empty-state
  checks — PASS;
- project shortcuts, workspace/operator, pane lifecycle, history keyboard,
  Timeline Space keyboard, and stage-label checks — PASS.

The run measured 133 renderer mutations, 31 server-authoritative mutations,
28 raw dispatches, 464 facade dispatches, 457 exact Tauri commands, 3689/3689
localized static UI entries, 15552 shortcut matrix cases, 28 workspace/operator
assertions, 243 build-wrapper assertions with 27 hostile fixtures, and no
unprotected bare user-data labels. Native three-screen layout, viewport and
scaling, screen-reader/high-contrast/IME behavior, and native keyboard-only
acceptance remain unclaimed.
