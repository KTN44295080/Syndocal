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

## Continuation — high-DPI Video upper-desk regression repair — 2026-09-15

The current user screenshot is a physical `2560x1504` Windows work area. On
the current high-DPI host, that surface is presented to the WebView at roughly
`1280x752` CSS pixels. The existing `@media (max-height: 800px)` Video reflow
therefore selected its intentional low-DPI operating-floor `36% / 64%`
monitor/lower split, which made the Preview/Program surface visibly too short
despite the large physical monitor.

The product repair is limited to the Video desk in `app/src/styles.css`: when
the CSS surface is at least `1200px` wide and reports at least `144dpi`, the
short-height rule is overridden back to the monitor-first `60% / 40%` split.
The ordinary low-DPI `1280x720` short-height reflow remains unchanged. No
typography, button, hit target, output route, or outer workspace split was
changed.

The browser gate now includes the physical-large/high-DPI equivalent as the
`2560x1504-2x` case (`1280x752`, `deviceScaleFactor=2`) and requires the same
`48%` minimum upper-desk share as the normal-height cases. The existing
`1280x720` case continues to use the `25%` short-height floor.

Rendered verification used the explicit installed Chrome fallback because the
Browser plugin is unavailable in this session:

```text
pnpm.cmd --dir app run check:control-upper-workspaces
check:control-upper-workspaces passed:
  3840x2160, 2560x1440, 2560x1504, 1920x1080, 1280x720, 2560x1504-2x
2560x1504-2x equivalent: CSS 1280x752, Lighting upper=327px, Timeline upper=277px
1280x720 low-DPI floor: Lighting upper=295px, Timeline upper=245px
all cases: zero runtime exceptions, console/log errors/warnings, and harness errors
```

The generated high-DPI-equivalent Video screenshot was visually inspected at
`C:\TEMP\syndocal-control-ui-checkpoints\control-video-1280x752.png`; Preview,
Program, Preview Transport, and Master retain a readable primary surface while
the lower Clips/Outputs/Layers region remains contained.

The current source was rebuilt through the exact Windows native wrapper:

```text
pnpm.cmd --dir app tauri build --no-bundle
Finished release profile [optimized] target(s) in 3m 03s
SHA-256: 01C5F39A056F781DA74BA5DB138318AD190299469CDD77FA66E47CB63059C5E7
Exact target/release/syndocal.exe: PID 6512, title Syndocal, Responding=True
```

This is current-source rendered-browser geometry plus exact native build and
process-smoke evidence. It does not establish native button-by-button
interaction, native accessibility, the current three-display acceptance gate,
physical output, external clients, recovery, venue/soak, signing, publication,
or product completion. `UI-H5-CONTROL-001` remains `Open`.

## Continuation — current three-display preflight recheck — 2026-09-15

The read-only three-display preflight was rerun after the latest display
change:

```text
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File qa/harnesses/run-syndocal-three-display-show-acceptance.ps1 -EvidenceSlug current-three-display-preflight-20260915c
verdict: not-configured
accepted: false
```

The live DisplayConfig inventory still contains exactly two connected
monitors. `DISPLAY1` is `2560x1600` with work area `2560x1504` and effective
DPI `192`; `DISPLAY2` is `1920x1200` with work area `1920x1152` and effective
DPI `96`. The expected third stable monitor identity is absent, so the
harness did not apply settings, move or maximize windows, create outputs,
touch hardware, inject input, or claim any three-display acceptance.

`UI-H5-CONTROL-001` remains `Open`. The next safe native three-display step is
to rerun the same read-only preflight once Windows exposes the third monitor,
then bind the exact stable identities and current release artifact before any
output-window acceptance. The missing monitor is an environment boundary, not
a product pass.

## Continuation — current release native visual recheck — 2026-09-15

The exact running `target/release/syndocal.exe` was captured through the
repository's read-only window harness:

```text
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File qa/harnesses/capture-window.ps1 -TitlePattern '^Syndocal$' -OutPath C:\TEMP\syndocal-control-ui-current-20260915.png -SettleMilliseconds 1200
captured 'Syndocal' (2586 x 1530)
SHA-256: AFFFE6DFBD337A6707946A8BA02E1CABDB8F73105974D6FE45B041855273ED02
```

Visual inspection of the captured native window confirms that the repaired
high-DPI Video upper desk keeps Preview, Program, Preview Transport, and
Master readable, while the lower Clips, Outputs, and Layers region remains
contained without overlap. The capture only foregrounded and photographed the
already-running exact-path window; no DOM/native click, dangerous Control
action, recording, device action, or output was dispatched. The Computer Use
surface exposed no native app target during this observation, so no coordinate
or guessed-HWND automation was attempted.

This is current-source native visual evidence for the layout repair only. It
does not establish current-source H5 acceptance, native button-by-button live
workflow, accessibility, physical output, external clients, recovery,
venue/soak, signing, publication, or product completion. `UI-H5-CONTROL-001`
remains `Open`.

## Continuation — current rendered upper-workspace recheck — 2026-09-15

At current source HEAD `38c4b45f`, the browser fallback was rerun with the
explicit installed Chrome executable. All six viewport cases passed:

```text
3840x2160: Lighting upper=1199px; Timeline upper=1153px; lanes=6; sources=4
2560x1440: Lighting upper=781px; Timeline upper=735px; lanes=6; sources=4
2560x1504: Lighting upper=818px; Timeline upper=772px; lanes=6; sources=4
1920x1080: Lighting upper=573px; Timeline upper=527px; lanes=6; sources=4
1280x720: Lighting upper=295px; Timeline upper=245px; lanes=6; sources=4
1280x752 high-DPI equivalent: Lighting upper=327px; Timeline upper=284px; lanes=6; sources=4
```

Every case passed first-Escape focus return and reported zero runtime
exceptions, console/log errors or warnings, and harness errors. The repair
therefore remains effective for the reported physical `2560x1504` surface and
its high-DPI equivalent; the low-DPI short-height floor remains intact. This
run used rendered browser fixtures only and did not click the running native
window, dispatch Control actions, access hardware, or contact an external
client.

The Q4 ledger records this rerun as
`EV-UI-H5-BROWSER-RERUN-CURRENT-2026-09-15`. `UI-H5-CONTROL-001` remains
`Open` because native button-by-button live/dangerous workflows, native
accessibility, physical output, external clients, recovery, venue/soak,
signing, and publication are still unaccepted.

## Correction — current three-display topology — 2026-09-15

A later read-only recheck confirmed that the current Windows desktop exposes
three connected displays. The earlier two-display observation is superseded
for the current host:

```text
DISPLAY1: 2560x1600, effective DPI 192, work area 2560x1504
DISPLAY2: 1920x1200, effective DPI 96,  work area 1920x1152
DISPLAY3: 3840x2160, effective DPI 144, work area 3840x2088
```

The same read-only harness returned `verdict: not-configured` and
`accepted: false` because its separate StandardRelease role profile expects
1920x1080 editor/LED surfaces and a 3840x2160 projector. No settings were
changed, windows moved, outputs created, or hardware touched. This is a
role-profile result, not evidence that a third display is absent and not a
reason to treat display size as the H5 workflow itself.

`UI-H5-CONTROL-001` remains `Open` only for the separately defined native
Control workflow evidence: live Lighting/Video/Audio, Cue/Clip/Take/
Transition, Blackout/Arm/Take Over, recording, diagnostics, failure/recovery,
and native interaction/accessibility proof. The three-display topology is now
recorded as observed supporting evidence, not as a substitute for those
workflow checks.
