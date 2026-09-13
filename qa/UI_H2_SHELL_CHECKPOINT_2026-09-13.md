# UI H2 shared shell checkpoint — 2026-09-13

## Scope

This checkpoint closes `UI-H2-SHELL-001` for the current source. It covers the
shared Setup/Edit/Control shell, persistent status/readout placement, operator
navigation labels and focus, ShowClock shell status, operator-lock boundaries,
and the browser pane-window contract. Internal route identifiers remain
`setup`, `control`, and `touch` for saved-layout and pane compatibility; the
operator-facing labels are Setup, Edit, and Control.

Branch: `codex/showclock-review-20260912`.

## Implemented behavior

- `WorkspaceChrome` now exposes exactly three peer workspace controls with
  `aria-current="page"` on the active item and stable operator-facing labels.
- The top bar contains a persistent fail-closed ShowClock chip. It reports
  `Unchecked`, `Stopped`, `Acquiring`, `Locked`, `Hold`, `Stale`, or
  `Unavailable` from the shared status model; missing or failed native status
  reads do not present a healthy state.
- ShowClock status-panel and top-bar summaries use the same shell mapping.
- F-key navigation reports display labels, preserves visible focus, rejects
  Setup while operator lock is active, and keeps locked programming routes
  fail-closed.
- Full Lock retains only the two current safety controls: emergency S0
  all-output engage and the disabled `BLACKOUT RELEASE LOCKED` indicator.
  The older three-button expectation was stale after safety-blackout separation
  and was corrected in the viewport contract; no release control was restored.
- The independent pane-window route verifies stage, timeline, programmer,
  setup, live, mixer, and touch roots with the top bar hidden and zero outer
  scroll. The harness recycles the browser only between independent protected
  route phases so a fixture close guard cannot mask the pane contract.

## Evidence

All commands below exited 0 on the current source:

| Gate | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:workspace-operator` | 40 assertions passed |
| `node app/scripts/check-workspace-navigation-controller.mjs` | Back/forward, guard, branching, epoch/rejection, project-fence and cleanup contract passed |
| `pnpm.cmd --dir app run check:workspace-operator-viewport` | 5/5 browser viewports passed: 1920x1080, 1920x1032, 2048x1152, 1366x768, 1280x720 |
| `pnpm.cmd --dir app run check:frontend-invokes` | 475 commands |
| `pnpm.cmd --dir app run check:frontend-command-routing` | 133 renderer, 31 server-authoritative, 28 raw, 481 facade dispatches |
| `pnpm.cmd --dir app run check:empty-states` | Passed |
| `pnpm.cmd --dir app build` | TypeScript/Vite passed; existing Vite chunk-size advisory remains (App 502.05 kB) |
| `pnpm.cmd --dir app tauri build --no-bundle` | Passed with pinned MSVC 14.44.35207 Build Tools linker |

The exact native executable was launched after the build. Observed:

```text
sha256=38C4E36E28522CCB3D5E458ED55196EB820560945D17670E8B1D2B71C8B5E43D
exact_process_count=1
window_count=1
title=Syndocal
responding=true
native_window_smoke=PASS
```

The exact executable path was stopped afterward and no matching process
remained.

## Boundaries and next action

This closes the current-source H2 shared-shell slice only. It does not claim
native button-by-button accessibility, NVDA, High Contrast, text scaling,
native dialog/popout focus, physical HDMI/DMX/MIDI output, external clients,
real two-machine ShowClock operation, venue rehearsal, signing, publication,
or H3/H4/H5 completion. The next ordered marker is `UI-H3-SETUP-001`.
