# DMX Patch layout correction — 2026-10-22

Base: `main` at `0939eb251ae595af1644ef9f6626785e089b68a9`.

Status: accepted. Product, browser matrix, maintained Patch regressions,
independent review, and Windows native acceptance passed.

## Reproduction and cause

The actual App's Setup > Lighting > Patch surface was reproduced with the
maintained Patch fixture and its real workspace ancestors. Evidence lives in
`target/qa/dmx-patch-layout-2026-10-22/`.

- At 1280×752, DPR 2, 100% scale, the map's computed row heights were
  `68px 0px 163.516px`. The selection row had zero height. Select All,
  Clear Selection, Move to address, and Move Selected were covered by the
  address grid; all four center hit checks failed.
- At 960×752, DPR 2, 110% scale, List disappeared beyond the map's right
  boundary, Next Free was clipped at the top, and Universe/Overview were
  truncated. The Patch Source search area was also vertically clipped.
- The 1920×1080, DPR 1, 100% sample preserved the controls' center hits;
  the failure depended on the actual narrow Patch pane, not just the window.

The Patch-specific high-specificity two-row grid declaration overrode the
later three-row declaration for header, selection controls, and grid/list.
The Patch header also forced `nowrap` while its enclosing panels clipped
overflow. The Overview popup used viewport-based dimensions rather than its
actual containing map. The address-grid height calculation mixed CSS pixel
sizes with rectangles after UI zoom.

`before-02/` contains closed/Overview PNGs and geometry JSON for all three
sizes. Earlier capture attempts are preserved. They timed out because the
fixture initially opened Edit; explicit Setup > Lighting > Patch navigation
fixed that capture harness assumption before any product change.

## Corrected behavior

The Patch map now reserves separate rows for its header, selection controls,
and address grid/list. Header actions wrap without reducing their control or
text sizes. At AddressDesk widths up to 450 CSS px, an explicit Selection and
move disclosure presents the same selection controls in a scrollable popup.
Opening it does not move the grid or its drop targets. Wider panes keep those
controls directly visible. Selection and Overview close one another; Escape
closes selection from either its toggle or its controls and returns focus.
Overview is bounded by the actual Patch pane. Grid sizing and keyboard address
reveal use consistent CSS pixel units under UI zoom.

The existing Patch Source browser already lets its search and GDTF Import
controls be reached through internal scrolling. Expanded Placement options
needed a separate correction: only the expanded form receives a bounded
scrollport, retaining the normal closed form and all input/control sizes.

## Validation

`interaction-final-02/evidence.json` records the final product hashes and five
successful App fixture cases. Every recorded check is true; errors and failures
are empty. Its product hashes match the native build's source hashes.

| Viewport | DPR | UI scale | Checks | Visible grid height |
| --- | ---: | ---: | ---: | ---: |
| 1280×752 | 2 | 100% | 24 passed | 84.52 px |
| 1280×752 | 2 | 110% | 24 passed | 48.53 px |
| 960×752 | 2 | 100% | 28 passed | 110.52 px |
| 960×752 | 2 | 110% | 28 passed | 86.55 px |
| 1920×1080 | 1 | 100% | 24 passed | 425 px |

These cases cover header and selection controls through Move Selected, preserved
control dimensions, both Escape paths, mutually exclusive popups, all three
Overview legends with matching editor/selection state, Grid/List, keyboard
address 1/512 reveal, Source search/Import/profile access, and all 12 expanded
Placement controls. Search and the close affordance remain reachable while
Placement is open. Root/app/workspace scroll offsets stay unchanged.

The 960/110% case also passes wide input focus → compact and compact open → wide
→ compact transitions: focus moves to the visible toggle when required, while
selection count and expansion state survive. Closed, popup, and scrolled-to-end
screenshots were visually inspected. Independent review found no blocking issue
in the frozen product diff and final five-case evidence.

TypeScript `--noEmit` passed. The maintained Patch viewport gate now includes
these five cases and strict ancestor clipping, center-hit, sizing, and interaction
checks without removing existing assertions. `maintained-patch-03/` records its
exit 0: the existing 1280×752 continuous grid/keyboard/A497 checks, profile DnD
(3 → 4 fixtures, conflict rejected without an extra fixture), GDTF Share, empty
state, and all five added layout cases pass.

`pnpm --dir app run check:fixture-repatch:browser` also passed: trusted pointer
drop at 33, group offset drop at 97, List Move at 80, pointer cancellation with no
apply, stable DOM after selection/drag, and no errors. The fixture tests simulate
authoring and do not establish physical DMX behavior.

The maintained gate uses an isolated, warmed dev server. Earlier failed attempts
are preserved: the cold server missed the initial mount deadline, then the
test's one-pass nested scrolling left 2 px of unused scroll range. The scroll
helper now remeasures at most twice, moving only auto/scroll ancestors. The
clipping threshold, hit checks, and existing product assertions were retained.

### Windows native acceptance

`pnpm --dir app tauri build --no-bundle` passed with the required MSVC
14.44.35207 Build Tools linker pinned and first on PATH. The standard build
identifier is `jp.seraf.ktn.syndocal`; source hashes stayed stable during the build.
Rust warnings remain 0 → 0. The existing Vite chunk-size warning remains 1 → 1
(delta 0). No temporary ISF trace is present.

Updated executable: `target/release/syndocal.exe`.

SHA-256: `cd4c3a1f8d3048a9407d086de48d44be37c2711295bfd91f48038cd2cc12516c`.

`native/build-01.json`, `native/launch-01.json`, and
`native/native-patch-final01.json` bind the source, binary, process, and UI evidence.
Exactly one responsive checkout window (PID 36548) was verified and maximized
before UI actions. At 1280×752, DPR 2, UI scale 100%, all nine header/selection
controls fit their viewport and clipping ancestors, retain their text, and pass
center-hit checks. Grid and List both occupy a visible 86.02 px scrollport;
the grid contains all 512 addresses. Actual clicks switch Grid/List and open/close
Overview. Overview fits the map and viewport without horizontal overflow.
Both native screenshots were visually inspected.

Navigation was restored, and the task-owned process closed normally
(`native/close-01.json`). The existing standard profile remained Standby with
lighting/video permissions denied; role and extension file hashes were unchanged.
Native acceptance used the empty standard project. Populated interactions are
covered by the isolated browser evidence above; no physical DMX output was tested.

## Observed boundaries

Move Selected reaches the existing fixture-overlap rejection in the isolated
browser fixture (`Selected fixture footprints overlap`). That proves the reachable
control still enters its existing validation; it is not evidence of a successful
authored patch write or of project-authority validation. Short adjacent
fixtures can retain overlapping proportional Overview segments at their existing
minimum hit size. All corresponding legend buttons are reachable and selectable;
the track check covers vertical containment and preserved button height.

After compact-open → wide, the first Escape clears the remembered selection
expansion. If global Learn is active, its Escape cancellation can require the
next press. Existing grid drag cancellation runs first. Independent review treated
this as a non-blocking interaction boundary for this layout correction.
