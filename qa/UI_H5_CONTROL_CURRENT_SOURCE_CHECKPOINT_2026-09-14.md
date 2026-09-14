# UI-H5-CONTROL-001 current-source checkpoint — 2026-09-14

- Marker: `UI-H5-CONTROL-001`
- Branch: `codex/showclock-review-20260912`
- Base: `92d3b838bcbcb657889b1e955750aa4710382987`
- Product code change in this checkpoint: none. The owned change is the native
  QA checker only; the existing Control Both blackout semantic change remains
  the product-source baseline.

## Existing H5 implementation evidence

The current H5 implementation includes the combined `Both` overview and the
lease-bound Lighting/Video master, cue, blackout, clip, Take, and launch paths.
The previously recorded `EV-UI-H5-BOTH-SW-2026-09-14` evidence covers its
TypeScript/Vite, routing, rendered 1920x1080 browser, and pinned Windows native
build/process-smoke checks. The exact process-smoke artifact recorded there is
the unsigned current-source executable with SHA-256
`16A853E5F1AB38C97918854B436A2CC8A9B15CF8AA72732BA16DDB2C8A7456AC`.

The previously recorded evidence remains the implementation baseline for
layout and routing. The latest source change adds semantic toggle state to the
three Both blackout controls. The detailed implementation record is
`qa/CONTROL_BOTH_CHECKPOINT_2026-09-14.md`.

The latest source was rebuilt after that change with the pinned MSVC
`14.44.35207` Build Tools linker. The resulting exact artifact is
`target/release/syndocal.exe`, 66,230,272 bytes, SHA-256
`5D2B479C39AB28CC10F2961EAB156E6EF8127E7571189BFF85D50871ACCDC66E`.
Exact-path process smoke found one PID (`27752`), title `Syndocal`, non-zero
window handle (`1444984`), and `Responding=True`; the exact-path process count
was zero after clean shutdown. This is native build/start evidence only: no
button-by-button Control action or physical output was exercised.

The current native maximized/F11 acceptance was then attempted with the
isolated QA executable. It failed closed at the first size gate because the
last observed maximized client was `1280x752`, below the required `1920x1000`.
The live Windows display query for this run reported one attached display at
`1280x800` with a `1280x752` work area, so the required `1920x1080` monitor was
not available. The QA process and ports were cleaned up; this is an environment
boundary, not a passing native interaction result.

## Takeover native gate retry — 2026-09-14

The native gate was rerun from the current source after the host display
changed. The GPU query reported `2560x1600`, but the app-owned maximized client
still measured `1280x752`; the gate therefore failed closed at
`Wait-ForMinimumClientDimensions` before F11 or any UI interaction. The user
display registry reported `AppliedDPI=192` (200%); this explains the available
logical work area but does not satisfy the required `1920x1000` client gate.
The current attempt used the pinned MSVC `14.44.35207` wrapper and exited with
code `1`; no native interaction or physical output is claimed. This failed
attempt remains historical evidence of the checker failing closed under a
DPI-virtualized host measurement.

## Takeover native pane acceptance — 2026-09-14

The checker was then corrected to establish Per-Monitor V2 before reading
Win32 monitor and client dimensions. It also suppresses CDP task acknowledgements,
accepts intentional empty pane arrays, distinguishes the exact
`tauri-plugin-single-instance` helper window from user-facing panes, applies the
decorated child-pane client minimum, retries verified maximize/foreground
operations within bounded time, reaps both isolated dev ports after restart,
and reconciles only stale Stage/Timeline records belonging to the exact QA PID.
The checker remains fail-closed for every unknown title, PID, or foreground
window.

The formal current-source run temporarily changed the primary display to
`1920x1080@180Hz/32bpp` with the reversible Win32 display-settings wrapper and
restored the original `2560x1600@180Hz/32bpp` mode in `finally`. The report is
`%TEMP%\syndocal-native-acceptance-20260914-012135\native-window-acceptance.json`.
It passed with monitor/work area `1920x1080` / `1920x1008`, main maximized
client `1920x1008`, F11 client `1920x1080`, and Esc-restored maximized client
`1920x1008`. The same run passed Stage-first and Timeline-first detach orders,
both pane rejoin paths, detached-pane restoration after process restart, exact
child adoption after main-window reload, direct Stage-child close reintegration,
and final full reintegration. It wrote 19 screenshots and exited with code 0;
the wrapper restored the original display mode successfully.

This is current-source Windows native main-window and Stage/Timeline pane
geometry/lifecycle evidence only. It does not establish native Control
button-by-button interaction, native accessibility, physical MIDI/OSC/DMX/
Art-Net/sACN/USB/RDM/video/display output, external clients, two-machine
operation, venue/soak, signing, publication, or product completion. The two
explicitly unverified native record-retirement boundaries in the JSON report
remain unverified.

## Takeover native Control surface probe — 2026-09-14

The same current-source native run then exercised the safe, non-output Control
surface in the real maximized window at the temporarily selected physical
`1920x1080@180Hz/32bpp` display. The report is
`%TEMP%\syndocal-native-h5-20260914-053000\native-window-acceptance.json` and
contains four additional Control screenshots (`control-edit.png`,
`control-mixer.png`, `control-both.png`, and `control-live.png`).

The probe passed the exact four-tab census (`edit`, `mixer`, `both`, `live`),
ARIA `role=tab`/`aria-selected`/`aria-controls` semantics, one visible panel
per mode, and zero document/app scroll. It reached the Lighting surface and
its view actions; the full Video desk including Preview/Program, Clip and
transition, Layers and outputs, Clip Slot bank, diagnostics, and the safe
Audio In drawer; the empty-show recording bar's explicit empty guard; the
Both cards, two monitors, Back/GO/Release, and three semantic blackout states;
and the Timeline arranger plus its lower-right Source shelf. The probe only
navigated and observed safe structure/state. It did not dispatch output,
recording, Take, blackout, Arm, Take Over, or device actions.

The Timeline live-status rail is intentionally hidden in the shared native
shell while the arranger owns the upper surface, and the Source shelf is
owned by the persistent lower-right context pane. The checker now measures
those actual ownership boundaries rather than treating either as a missing
product surface. Display restoration to the original
`2560x1600@180Hz/32bpp`, isolated-port release, and QA-process cleanup were
confirmed after the run.

This is native Control surface structure and semantic-state evidence only. It
does not close native button-by-button interaction, full live workflow,
accessibility, physical output, failure/recovery, external-client,
venue/soak, signing, publication, or product completion gates.

## Acceptance boundary

`UI-H5-CONTROL-001` remains `Open`. The current native window/pane gate now has
current-source evidence on a physical `1920x1080` display, and the safe Control
surface probe now covers native reachability/semantic structure. The existing
evidence still does not establish native button-by-button interaction, full
live Lighting/Video/Audio workflow completion, native accessibility, physical
output, failure/recovery rehearsal, external clients, venue/soak, signing,
publication, or product completion.

The fresh browser rerun used the current Chrome executable through
`CHROME_PATH=C:\Users\janua\AppData\Local\Google\Chrome\Application\chrome.exe`.
The Control upper-workspaces gate passed at all four supported viewports:
`3840x2160`, `2560x1440`, `1920x1080`, and `1280x720`. It exercised the
Lighting, Video, Both, Timeline/nested-controls reachability, first-Escape
focus return, and final CDP diagnostics; every viewport reported zero runtime
exceptions, console/log errors or warnings, and harness errors. This is fresh
rendered browser evidence for the semantic assertions. Native interaction and
physical output remain separate unaccepted boundaries.

## Source continuity audit — 2026-09-14

The blackout semantic change was made in `894c690c`. A current-tree audit with
`git diff --name-only 894c690c..HEAD -- app crates` returned no paths, so the
current product source still matches the source used for the post-change
native build/process smoke and the recorded static checks. The native pane
acceptance above is an additional checker/runtime QA result and does not turn
the remaining native Control, accessibility, or external UI boundaries into a
pass.

## Resume procedure

Run the H5 matrix on the exact current artifact: live Cue/Clip/Take/Transition,
Blackout/Arm/Take Over, recording and diagnostics, failure/recovery, and native
interaction at the required operator viewports. Preserve the first failure and
separate native, physical, and venue evidence before changing the marker.

## Takeover browser rerun — current HEAD — 2026-09-14

The repository's existing Chrome/Playwright fallback gate was rerun against
HEAD `70c7fb30` because the Browser plugin is not available in this session.
It passed all four supported viewports: `3840x2160`, `2560x1440`, `1920x1080`,
and `1280x720`. Each viewport passed Lighting and Timeline layout assertions,
first-Escape focus return, and final CDP diagnostics with zero runtime
exceptions, console/log errors or warnings, and harness errors. The script
reported `check:control-upper-workspaces passed` and cleaned its browser tree,
Vite server, endpoint, and profile.

This is a fresh rendered-browser interaction result only. It does not change
the native button-by-button, dangerous-action, accessibility, physical-output,
external-client, venue, signing, or publication boundaries above.

## Takeover native artifact/process recheck — 2026-09-14

At source HEAD `f9ea7810`, `pnpm.cmd --dir app tauri build --no-bundle`
completed with the pinned Build Tools MSVC `14.44.35207` linker first in
`where.exe link.exe`. The resulting exact checkout artifact was
`target/release/syndocal.exe`, `66,230,272` bytes, SHA-256
`C95CD9F832FF2E12E25FC132C87E7E2EC44BE11A27348AA94542FE20EC45A940`.

An exact-path process smoke then found one `Syndocal` window with a non-zero
window handle and `Responding=True`; cleanup terminated only that exact
artifact path. The standard `check:native-window` acceptance was also
attempted, but failed closed before window interaction because the current
primary monitor is `2560x1600` and the required `1920x1080` mode is not
available in the enumerated modes. No output, recording, Take, blackout, Arm,
Take Over, or device action was performed. This adds current artifact and
process evidence only; `UI-H5-CONTROL-001` remains `Open` for the required
native control workflow, accessibility, and physical/external acceptance.

## Supplemental native run on the available display — 2026-09-14

Because the required `1920x1080` mode is unavailable on this host, the same
checker was run against the current physical display without changing the
product or weakening the formal gate: `-MinimumMaximizedClient 2400x1500`
and `-ExpectedFullscreen 2560x1600`. The generated report was
`%TEMP%\\syndocal-native-acceptance-20260914-045044\\native-window-acceptance.json`.
It recorded maximized `2560x1504`, F11 `2560x1600`, exact restore,
Control Lighting/Video/Both/Timeline semantic-state acceptance, both pane
detach orders, restart restoration, reload adoption, direct Stage-child close
reintegration, and final full reintegration. The report's observable checks
all emitted `PASS`, and the acceptance wrapper exited `0` after the owned
Tauri dev subtree was reaped. The subtree still printed its expected
`ELIFECYCLE` shutdown line while it was intentionally terminated; that line
does not replace the wrapper's successful result. This is therefore still
supplemental evidence, not a formal gate pass, because the run used the
available-display dimensions rather than the required `1920x1080` mode.
No output, recording, Take, blackout, Arm, Take Over, or device action was
performed. `UI-H5-CONTROL-001` remains `Open`.

The checker status fix is intentionally narrow: after a successful assertion
path, it explicitly exits `0` after `finally` cleanup so `taskkill.exe` cannot
become the script status; assertion failures still throw and remain non-zero.

## Formal 1920 native gate after secondary display installation — 2026-09-14

After a secondary display was connected, the live DisplayConfig inventory
identified `\\.\DISPLAY2` as `1920x1200` at effective DPI `96`, with
`1920x1080` available in its enumerated modes. The reversible test wrapper
recorded the original `1920x1200@165Hz`, changed only DISPLAY2 to
`1920x1080@60Hz`, moved the isolated QA window to that monitor, and restored
the original mode in `finally` with Win32 result `0`.

The formal checker then completed with wrapper exit `0`. Report:
`%TEMP%\\syndocal-native-acceptance-20260914-082019\\native-window-acceptance.json`.
It records monitor `1920x1080` / work area `1920x1032`, maximized client
`1920x1032`, F11 client `1920x1080`, exact Esc restoration, all four safe
Control Lighting/Video/Both/Timeline semantic-state checks, both pane detach
orders, restart restoration, main-window reload adoption, direct Stage-child
close reintegration, and final full reintegration. All observable assertions
printed `PASS`; the intentional owned Tauri subtree cleanup still emits its
`ELIFECYCLE` line, but the acceptance wrapper returned `0`.

This closes the current-source native display/pane-lifecycle sub-gate only. No
output, recording, Take, blackout, Arm, Take Over, device, external client,
screen-reader, High Contrast, scaling, IME, or venue action was performed.
`UI-H5-CONTROL-001` remains `Open` for the complete live/dangerous Control,
accessibility, physical-output, external-client, and recovery requirements.

## Takeover continuation — current-source browser H5 upper-workspace recheck — 2026-09-14

At current source HEAD `b5f1498d`, the existing Chrome/Playwright fallback gate
was rerun with the explicit installed Chrome executable because automatic
browser discovery did not select a usable browser:

```text
$env:CHROME_PATH = 'C:\Users\janua\AppData\Local\Google\Chrome\Application\chrome.exe'
pnpm.cmd --dir app run check:control-upper-workspaces
check:control-upper-workspaces passed: 3840x2160, 2560x1440, 1920x1080, 1280x720
```

All four viewports passed Lighting and Timeline containment, first-Escape
focus return, and final CDP diagnostics with zero runtime exceptions, console
or log errors/warnings, and harness errors. The browser tree, Vite server,
endpoint, and temporary profile were cleaned after each viewport.

This is rendered-browser upper-workspace evidence only. It does not prove
native button-by-button interaction, dangerous actions (Take, Blackout, Arm,
Take Over, recording), native accessibility, physical output, external
clients, venue/soak, signing, or publication. `UI-H5-CONTROL-001` remains
`Open` for those boundaries.

## Continuation — Video upper-desk repair and rendered recheck — 2026-09-14

The current user-provided `2560x1504` screenshot exposed the Video
Preview/Program row as too vertically compressed. The product CSS repair was
made in `app/src/styles.css` and rebuilt into the exact release executable;
the main, native mixer pop-out, and large fullscreen rules now allocate
`60% / 40%` to the monitor row versus the lower Clip / Outputs+Layers row.
The existing `max-height: 800px` `36% / 64%` operating-floor reflow remains
unchanged. No typography, control, hit target, or output behavior was altered.

The existing browser checker was strengthened in commit `1eb669cd` to measure
the actual `.videoMixerTopPane` and `.videoMixerTopContent`. It now requires a
minimum upper-desk share of `48%` at normal heights and `25%` at the existing
short-height floor, in addition to requiring visible Preview/Program content.

Rendered verification used the explicit user-installed browser because the
Browser plugin is unavailable in this session:

```text
$env:CHROME_PATH = 'C:\Users\janua\AppData\Local\Google\Chrome\Application\chrome.exe'
pnpm.cmd --dir app run check:control-upper-workspaces
check:control-upper-workspaces passed:
  3840x2160, 2560x1440, 1920x1080, 1280x720
```

All four viewports passed the new Video upper-desk geometry/content checks,
the existing Lighting/Video/Both/Timeline reachability and containment
checks, first-Escape focus return, and final CDP diagnostics. Every viewport
reported zero runtime exceptions, console/log errors or warnings, and harness
errors. Screenshots were written outside the repository under
`C:\TEMP\syndocal-control-ui-after-20260914`; the 2560x1440 and 1920x1080
Video screenshots were visually inspected and show the expanded
Preview/Program surface with the lower Clip, Outputs, and Layers panes still
contained.

`UI-H5-CONTROL-001` remains `Open`: this continuation proves the rendered
layout repair and safe browser reachability only. Native button-by-button
dangerous Control actions, native accessibility, physical output, external
clients, recovery rehearsal, venue/soak, signing, and publication remain
unaccepted.

## Continuation — exact user-reported Video viewport regression gate — 2026-09-14

The browser gate now includes the user's exact `2560x1504` screenshot surface
in both the focused checker and its aggregate runner. This closes a test
coverage gap: the earlier `2560x1440` result could not by itself prove the
reported client dimensions. No product typography, control size, hit target,
or output behavior was changed in this continuation.

Because the Browser plugin is unavailable in this session, the explicit
installed Chrome executable was used for the Playwright/CDP fallback:

```text
pnpm.cmd run check:control-upper-workspaces
check:control-upper-workspaces passed: 3840x2160, 2560x1440, 2560x1504, 1920x1080, 1280x720
```

At `2560x1504`, the rendered browser evidence reported the Video desk's
measured upper share above its `48%` minimum, Lighting upper height `818`px,
Timeline upper height `768`px, six Timeline lanes, four source cards, and
first-Escape focus return. Across all five viewports, final CDP diagnostics
reported zero runtime exceptions, console errors/warnings, log errors/warnings,
and harness errors; outer document/app scroll remained fixed. The generated
`control-video-2560x1504.png` and `control-video-import-open-2560x1504.png`
were visually inspected: Preview/Program is readable, the import disclosure
remains contained and reachable, and the lower Clip/Outputs/Layers region is
not overlapped.

This remains rendered-browser evidence only. Native dangerous Control actions,
native accessibility, physical output, external clients, recovery rehearsal,
venue/soak, signing, and publication remain unaccepted, so
`UI-H5-CONTROL-001` stays `Open`.

## Continuation — current three-display native gate preflight — 2026-09-15

The exact release artifact was already launched separately and verified as one
responsive `Syndocal` process. A fresh `pnpm.cmd run check:native-window` was
then attempted after the user's secondary-display installation. The checker
failed closed before native UI interaction because the current primary monitor
reported `2560x1600`, while this acceptance gate requires a physical
`1920x1080` monitor. The isolated QA process and Vite listener were released;
no Control action, output, recording, device, or external client was touched.

This does not invalidate the previously recorded reversible `1920x1080`
display-mode run, which passed the native pane lifecycle and safe Control
surface probes and restored the original `2560x1600` mode. It records that the
current three-display setup was not itself treated as proof of the required
primary 1920x1080 acceptance geometry. `UI-H5-CONTROL-001` remains `Open` for
native live/dangerous interaction, accessibility, physical output,
external-client, recovery, venue, signing, and publication evidence.
