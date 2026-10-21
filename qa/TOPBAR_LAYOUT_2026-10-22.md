# Responsive header layout checkpoint

Base: `main` at `f6c5fad38f4b1b0d06bf9467076a1828064bca3d`.
Scope: the reported clipped/overlapping upper UI, together with completion of
the inherited [native ISF observation](FX_DISPLAY_PERFORMANCE_2026-10-22.md#native-phase-observation-and-takeover-checkpoint).
This does not change transport, blackout, ownership, project, or window actions.

## Behavior

The header previously forced navigation, project identity, operator controls,
tempo context, clock status, and window controls onto one 40px grid row. Its
tempo labels used two 14px rows despite taller text, and the Clock badge had a
146px maximum width. This caused clipped labels and insufficient space for the
operator controls at common window widths.

`WorkspaceChrome.css` gives widths up to 1500 CSS pixels two explicit rows:
navigation/project/window controls first, operator controls second. Wide windows
retain one row. Controls/readouts may wrap when needed. Each grid row has a
40px minimum and can grow to fit its contents. Tempo rows now use their actual
line height, and the Clock badge can fit its text. Project identity and beat
lights remain visible at the native 960px minimum. Font, icon, button, and hit
target sizes were not reduced. At the tested 100% scale, the header is 90px tall
at constrained widths and 42px on wide desktops; the content area gives up the
corresponding 48px rather than losing controls.

The added header height also exposed a fixed-height assumption in the upper
Timeline Tools popup. Its available height now comes from the smaller of the
viewport and containing layout's actual bottom, with 8px clearance and CSS zoom
conversion. Existing internal scrolling and 44px controls are retained. Only
the open upper popup observes relevant size/scroll changes; closing or unmounting
disconnects the observers/listeners and removes the measured CSS variable. The
lower hosts keep their existing upward-opening placement.

## Focused browser evidence

Evidence root: `target/qa/topbar-layout-2026-10-22/`.
`attempt-05/header-evidence.json` and its before/after PNGs bind the real Solid
components to the source hashes and reproduce the previous clipping using
isolated baseline CSS from `f6c5fad3`.

| Coverage | Result |
| --- | --- |
| Widths 960, 1100, 1280, 1366, 1500, 1501, 1920, 2560, 3840 | Passed |
| Normal and long Japanese project name / dirty / Partial lock states | Passed at all nine widths |
| Required controls visible, contained, non-overlapping and hit-testable | Passed |
| Clock/source/sync text containment and separate tempo rows | Passed |
| Existing control and hit-target dimensions | Preserved |
| Tools popup visibility, containment, keyboard dismissal and controls | Passed |
| Back/GO/transport/blackout/Tap/BPM fixture interactions | Passed |
| Learn MIDI/OSC/DMX at 960px and 1280px with Tools open | Passed |
| Japanese / 110% scale at 1280px and 1501px, normal and Partial lock | Passed |
| Browser console / page errors | 0 |

The supervisor inspected the 1280px before/after and Timeline popup images.
Independent static review found no definite P1/P2 issue and confirmed that assertions were changed
to the responsive contract rather than dropped. Existing Timeline expansion,
Live Desk, PULSE, and Control upper-workspace gates now verify explicit row
placement, two-dimensional overlap, containment, hit testing, minimum sizes,
and absence of horizontal overflow. Browser-only evidence does not establish
native output or device behavior.

The full upper-workspace check passed at 1280 x 752 / DPR 2, including Lighting,
Video, Both, and Timeline. `upper-1280-dpr2-03/` retains its screenshots and zoom
measurement. Timeline Tools ended at y=719 inside a layout ending at y=727;
at 110% these were 716.5 and 724.516. Bank, Cue Audio, and Phases end controls
remained reachable by internal scrolling. The final button was 44px high at
100% and 48.391px at 110%, and the measured height property was removed on close.
Independent review also checked observer cleanup, zoom conversion, preserved
upward placement, and the absence of a resize feedback loop.

Focused existing viewport gates passed at 1280 x 720: `topbar-pulse`,
`timeline-expansion`, and `live-desk-header`. Earlier failing attempts are kept:
the old 160px popup offset first exceeded the viewport and then the containing
layout; the final constraint measures both boundaries. No containment or minimum
hit-target assertion was removed to accept those failures.

The upper-workspace gate also passed at 1920 x 1080 / DPR 1. Final TypeScript
and all three changed runner syntax checks passed. `validation-summary.json`
records the six source/runner hashes, exact commands, completed terminal session
identities, exit codes, and observed output excerpts; successful checks were not
rerun solely to capture logs. Both upper-workspace runs reported zero runtime
exceptions, console/log errors or warnings, and harness errors.

## Final Windows native acceptance

The standard configuration was built with
`pnpm --dir app tauri build --no-bundle`, using the exact pinned Build Tools
MSVC 14.44.35207 linker. The final build completed in 3m 07s and its product
source hashes remained unchanged. The normal checkout executable now contains
both this UI repair and the committed ISF scratch reuse, without the temporary
trace module:

- Path: `target/release/syndocal.exe`.
- SHA256: `46553377765fe57506a215020ee3fbcce701104815db9563ce1b6fa682110a96`.
- Identifier: `jp.seraf.ktn.syndocal` (standard, not a QA identifier).
- Rust compiler warnings: baseline 0 / current 0 / delta 0.
- Existing Vite chunk notice: baseline 1 / current 1 / delta 0.

Evidence is in the native campaign
`target/qa/fx-perf-2026-10-22/native/88baa1b5-514c-47d4-8626-21b0729a59e5/`:
`build-final-02.json`, `launch-standard-ui-02.json`,
`native-header-standard-complete01.json`,
`native-popup-standard-complete01.json`, and their screenshots.
`final-standard-audit.json` binds the executable, current source hashes, native
acceptance records, browser validation, warning counts, and process cleanup.

The final executable launched as exactly one responsive Syndocal main window
(PID 28048), with maximization observed before UI actions. At 1280 x 752 CSS
pixels / DPR 2, both 40px header rows were present, all required controls were
contained and hit-testable, and Clock/source/sync text was unclipped. The real
Edit/Timeline Tools popup opened inside both viewport and layout bounds; its
last visible button retained 44px height and was hit-testable. Closing the popup
released its measured height property. The native fixture was the ordinary empty
project, so long populated Bank/Cue Audio/Phases scrolling is established by the
separate browser fixtures above, not by this native observation.

This UI check left both output domains denied throughout. A fresh standard
startup intentionally reports Failed/StartupDenied even for the saved Standby
role; the native probe accepts that exact canonical startup boundary or settled
Ready while still requiring Standby and both permissions denied. An earlier
probe's Ready-only assumption is preserved as failed evidence and did not alter
the product output policy. No output was armed for the UI checks. The exact
native verification process was closed normally after screenshots; the final
standard executable remains at the checkout path above.
