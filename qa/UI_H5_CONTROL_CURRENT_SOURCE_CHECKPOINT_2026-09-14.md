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

## Continuation — high-DPI Video upper-desk repair final checkpoint — 2026-09-15

The user-reported upper Video surface was repaired in the product stylesheet.
The ordinary short-height `1280x720` reflow remains `36% / 64%`; only a large
physical display exposed through a high-DPI CSS viewport (`min-width:
1200px`, `min-resolution: 144dpi`, `max-height: 800px`) receives the
monitor-first `60% / 40%` allocation. This keeps Preview, Program, Preview
Transport, and Master visible without changing typography, control sizes, hit
targets, output routing, saved workspace ratios, display count, or display
role assumptions. The change is in `app/src/styles.css`; the outer workspace
split and the three-display observation are not part of this UI repair.

Validation was run from the current branch after the source change based on
`f3bc74c2`:

```text
pnpm.cmd --dir app run build
  PASS: TypeScript and Vite build, 358 modules transformed.
  Existing Vite advisory: some chunks exceed 500 kB; no new first-party
  compiler/runtime warning was introduced.

CHROME_PATH=C:\Users\janua\AppData\Local\Google\Chrome\Application\chrome.exe
pnpm.cmd --dir app run check:control-upper-workspaces
  PASS: 3840x2160, 2560x1440, 2560x1504, 1920x1080, 1280x720,
        and 2560x1504-2x (CSS 1280x752, deviceScaleFactor=2).
  PASS: first-Escape focus return and final CDP diagnostics at all six cases;
        runtime exceptions, console/log errors and warnings, and harness
        errors were zero.

pnpm.cmd --dir app run check:native-window
  PASS: -MinimumMaximizedClient 2560x1500,
        -ExpectedFullscreen 2560x1600,
        report C:\TEMP\syndocal-native-h5-fixed-20260915-final3\native-window-acceptance.json.
  PASS: maximized 2560x1504, F11 2560x1600, Esc restore 2560x1504.
  PASS: native Control Lighting/Video/Both/Timeline safe surface and
        semantic-state probe; Stage/Timeline detach in both orders;
        restart-with-detached-records; main reload child adoption; direct
        Stage-child close reintegration; final full reintegration.
  PASS: pinned MSVC 14.44.35207 linker was printed and matched where.exe.
```

The final native screenshots show a readable Preview/Program monitor row and
contained lower Clip/Outputs/Layers panes at the current physical desktop.
The native checker action boundary remained safe-only: it did not dispatch
output selection, recording, Take, blackout, Arm, Take Over, or device
actions. This is current-source Video layout plus safe native structure and
pane lifecycle evidence; it does not close `UI-H5-CONTROL-001`. Live
Lighting/Video/Audio operation, dangerous workflow dispatch, native
accessibility, physical output, external clients, failure/recovery rehearsal,
venue/soak, signing, and publication remain the actual H5 residuals. The
three-display role-profile result remains separate supporting topology data,
not a blocker or substitute for those workflows.

## Continuation — all-screen UI clipping review and repair — 2026-09-15

This checkpoint covers the user-reported high-DPI upper-screen collapse and a
fresh visual review of the shared workspace surfaces. The checkout was at
`d72af1cb698a2c8ce5b3f91f142765d31d2278aa` before this working-tree change on
`codex/showclock-review-20260912`; the product changes and this checkpoint are
owned by the current task.

The review found and repaired three concrete presentation defects:

1. On the exact native release window at physical `2560x1504`, effective DPI
   `192` / scale `2` (CSS `1280x752`), the engaged Safety blackout label was
   longer than its fixed action button and painted into the adjacent DMX/VID/
   ALL controls. The action now shows the compact `SAFE` label, retains the
   explicit `Release safety blackout` accessible name, and has overflow
   containment as a rendering backstop. No safety action was dispatched during
   the repair check.
2. The default Touch Image tile is one shared grid row (`48px`) high. Its
   default icon/label stack painted the `Touch Stage` label through the tile
   border on the native high-DPI surface. One-row Image tiles now use a local
   compact layout; larger Image tiles keep their original scale. The Touch
   containment checker now asserts that the Image content and all children stay
   within the authored tile.
3. Timeline Follow runtime status had insufficient separation at the compact
   width, rendering as `Runtimetransitioning 50%`. The runtime details row now
   uses an explicit gap and right-aligned state label so the two texts remain
   legible.

The affected product files are `app/src/components/WorkspaceChrome.tsx`,
`app/src/components/EditableTouchSurface.tsx`, and `app/src/styles.css`. The
focused containment contract synchronization is in
`app/scripts/check-viewport-containment.mjs`: it reflects the actual six
non-interactive topbar drag surfaces and two read-only Live/status pills, and
adds the compact Touch Image containment assertion.

Validation on the repaired source:

```text
pnpm.cmd --dir app run build
  PASS: TypeScript/Vite build, 358 modules transformed.
  Existing Vite advisory: chunks over 500 kB; no new first-party warning.

CHROME_PATH=C:\Users\janua\AppData\Local\Google\Chrome\Application\chrome.exe
pnpm.cmd --dir app run check:control-upper-workspaces
  PASS: 3840x2160, 2560x1440, 2560x1504, 1920x1080, 1280x720,
        2560x1504-2x / CSS 1280x752 / deviceScaleFactor 2.
  PASS: Lighting, Video, Both, Timeline, first-Escape focus return, and final
        CDP diagnostics at every case; runtime/console/log/harness errors and
        warnings were zero.

pnpm.cmd --dir app run check:topbar-pulse
  PASS: 1280x720; overflow=0, six drag surfaces, zero interactive drag
        surfaces, Live/status controls=2.

pnpm.cmd --dir app run check:video-setup-viewport
  PASS: 1920x1080, 1920x1032, 2048x1152, 1366x768, 1280x720; all mapping,
        preview, Advanced-disclosure, scroll, dock, and containment checks.

node app/scripts/check-viewport-containment.mjs --setup-io-only
  PASS: 1920x1080, 1920x1032, 2048x1152, 1366x768, 1280x720; six setup
        cards, disclosure reachability, routing controls, and zero overflow.

node app/scripts/check-viewport-containment.mjs --patch-only
  PASS: continuous Patch, DnD, GDTF share, empty-state, and responsive-scaling
        cases across 1920x1080, 1920x1032, 2048x1152, 1366x768, and 1280x720.

pnpm.cmd --dir app run check:touch
  PASS: all five Touch viewports and Default Desk/Viewport Touch modes;
        zero page scroll and compact Image content containment.

pnpm.cmd --dir app tauri build --no-bundle
  PASS: current `target/release/syndocal.exe`; pinned MSVC 14.44.35207 x64
        linker matched `where.exe link.exe`.
  Release SHA-256:
        fab4c9bc89d60ff72398896419022477c0ef29c559b2ed5f692e50f991d1e61f

node app/scripts/run-native-window-acceptance.mjs -MinimumMaximizedClient 2560x1500 -ExpectedFullscreen 2560x1600 -EvidenceDir C:\TEMP\syndocal-native-ui-final-20260915
  PASS: evidence C:\TEMP\syndocal-native-ui-final-20260915\
        native-window-acceptance.json.
  PASS: maximized 2560x1504, F11 2560x1600, Esc restore 2560x1504;
        safe Control Lighting/Video/Both/Timeline surface; Stage/Timeline
        detach in both orders; restart with detached records; main reload
        adoption; direct Stage-child close reintegration; final reintegration.
  Safe native action boundary: no output, recording, Take, blackout, Arm,
  Take Over, or device action was dispatched. Two record-retirement boundary
  cases remain explicitly unverified in the report.

Exact release process observation:
  PID 59264; one exact-path `Syndocal` candidate; one visible titled main
  window; responding=true; maximized=true; client 2560x1504; monitor
  2560x1600; effective DPI 192; CSS viewport 1280x752.
```

The latest exact release screenshot is
`C:\TEMP\syndocal-ui-final-release-20260915.png`. The rendered Touch and
Timeline screenshots are under
`C:\TEMP\syndocal-touch-ui-after-20260915\` and
`C:\TEMP\syndocal-control-ui-checkpoints\`; visual inspection shows the
`Touch Stage` label fully inside its tile, separate topbar controls, and the
Timeline `Runtime` / `transitioning 50%` labels separated.

The browser-plugin surface was unavailable, so the rendered checks used the
repository's Playwright/CDP fallback with the installed Chrome executable.
The broad legacy `check:viewport` runner remains a harness/selector mismatch
(`data-control-mode-option="both"` is absent in that stale route); it was not
used to manufacture a pass. Cleanup emitted only intermittent `EBUSY` for
temporary Chrome `CrashpadMetrics-active.pma` files while the affected checks
still exited `0`.

The Q4 mirror now records this bounded review as
`EV-UI-H5-ALL-SCREEN-REVIEW-CURRENT-2026-09-15` (`q4_evidence=123`). This
evidence accepts current-source rendered containment, focus return,
diagnostics, exact release build/process smoke, and safe native pane
lifecycle. It does not close `UI-H5-CONTROL-001`: native button-by-button
live/dangerous workflows, native accessibility matrix, physical output,
external clients, failure/recovery rehearsal, venue/soak, signing,
publication, and product-wide completion remain separate gates. The observed
three-display topology and its role-profile result also remain separate from
this UI repair.

## Current-source recheck — all supported UI surfaces — 2026-09-18

This is a documentation-only current-source recheck at `3f4d2a3e` on
`codex/showclock-review-20260912`. No product code, layout rule, fixture, or
native action was changed. The flow under test was: app loads -> each shared
workspace/Setup surface renders -> the existing containment and focus checks
exercise the visible controls without dispatching live output.

The Browser plugin was unavailable, so the repository's CDP/Playwright-style
fallback used the installed Chrome executable at
`C:\Users\janua\AppData\Local\Google\Chrome\Application\chrome.exe`.
The first `topbar-pulse` and first `video-setup` attempts stopped before any
product assertion with `Syndocal app shell did not mount`; rerunning each
fixture alone (the latter with `SYNDOCAL_VIEWPORT_TRACE=1`) passed. No runtime
exception, console error/warning, or harness failure was reported by the
successful runs. This is recorded as a startup-harness race, not as a UI
failure.

Successful rendered checks:

```text
pnpm.cmd --dir app run check:topbar-pulse
  PASS: topbar-pulse-1280x720; overflow=0, controls/order checks passed,
        project menu Save/Load passed, drag/status checks passed.

pnpm.cmd --dir app run check:control-upper-workspaces
  PASS: 3840x2160, 2560x1440, 2560x1504, 1920x1080, 1280x720, and
        2560x1504-2x / CSS 1280x752. Lighting, Video, Both, Timeline,
        first-Escape focus return, and final CDP diagnostics passed at each.

pnpm.cmd --dir app run check:video-setup-viewport
  PASS: 1920x1080, 1920x1032, 2048x1152, 1366x768, and 1280x720;
        mapping/preview/Advanced disclosure/scroll/dock containment passed.

pnpm.cmd --dir app run check:touch
  PASS: DVC touch feature preset (10 assertions), Default Desk and Viewport
        Touch matrices at all five viewports; no page scroll or compact Image
        tile containment failure.

node app/scripts/check-viewport-containment.mjs --setup-dmx-only
  PASS: setup-io fixture at 1920x1080, 1920x1032, 2048x1152, 1366x768,
        and 1280x720; cards, DMX controls, disclosures, scroll, legacy routes,
        Art-Net visibility, logical routing, and remote disclosure passed.

node app/scripts/check-viewport-containment.mjs --setup-io-only
  PASS: the same five viewports; MIDI, OSC, Remote, disclosure, routing,
        card, and scroll checks all passed.
```

The fresh visual captures under
`C:\TEMP\syndocal-control-ui-checkpoints` were inspected for the reported
upper-screen collapse at the 1920x1080 and high-DPI-equivalent CSS 1280x752
surfaces. The topbar actions remain separated, Preview/Program/Transport and
Master stay contained, and the lower Groups/Stage/Inspector boundary does not
overlap the upper workspace. No additional clipping or unreadable wrapping was
found, so no presentation change is justified by this recheck.

This evidence is limited to current-source rendered UI geometry, reachability,
focus return, and diagnostics. It does not claim native button-by-button live
Lighting/Video/Audio, Take/Transition, Blackout/Arm/Take Over, recording,
native accessibility, physical output, external clients, failure/recovery,
venue/soak, signing, publication, or product completion. Therefore
`UI-H5-CONTROL-001` remains `Open`; this recheck only refreshes its safe UI
surface evidence.

## Current-source short-height operator reflow — 2026-09-18

The reported upper-area collapse was reproduced in the current browser fixture
at the compact `1366x768` and `1280x720` viewports. The VJ operator containment
probe found the third layer action outside the visible layer scrollport. The
first reflow attempt moved Outputs and Layers side by side, which exposed a
second real issue: the third output selector could fall outside the narrow
Outputs rail. The final repair gives the short-height Mixer context a bounded
`350px / 250px` Outputs/Layers split, preserves the high-DPI `2560x1504` /
CSS `1280x752` monitor-first exception, and fixes opened compact drawer rows at
the usable `56px` floor. Typography and hit-target sizes were not reduced.

The focused QA harness also received only state/readiness corrections: a fresh
operator-VJ CDP target no longer waits on an unmounted `about:blank` shell,
Fullscreen VJ restores Control > Mixer before measuring the Mixer surface, and
the broad Control loop restores Control after the Setup-owned live-audio gate.
These changes do not weaken containment assertions.

Successful current-source rendered checks after the repair:

```text
CHROME_PATH=C:\Users\janua\AppData\Local\Google\Chrome\Application\chrome.exe
pnpm.cmd --dir app run check:control-upper-workspaces
  PASS: 3840x2160, 2560x1440, 2560x1504, 1920x1080, 1280x720,
        and 2560x1504-2x / CSS 1280x752; zero CDP diagnostics.

pnpm.cmd --dir app run check:vj-operator
  PASS: en/ja at 1920x1080, 1920x1032, 2048x1152, 1366x768, and 1280x720;
        zero unsafe overflow and zero outside-rect controls after the repair.

pnpm.cmd --dir app run check:fullscreen-vj
  PASS: 1920x1080, 1366x768, and 1280x720; fullscreen shared grid,
        monitor balance, drawer height, and stateful audio contract passed.

pnpm.cmd --dir app run check:audio-reactive-viewport
  PASS: 1920x1080 and 1366x768; SAFE ZERO strip contained, with the compact
        strip reduced from the reproduced 62px row to the bounded 56px row.

pnpm.cmd --dir app run check:live-audio-viewport
  PASS: en/ja at 1920x1080, 1920x1032, 2048x1152, 1366x768, and 1280x720;
        live state, safety clear, telemetry containment, and zero critical
        telemetry overflow passed.

pnpm.cmd --dir app run check:video-setup-viewport
  PASS: 1920x1080, 1920x1032, 2048x1152, 1366x768, and 1280x720.

pnpm.cmd --dir app run check:touch
  PASS: DVC preset (10 assertions), Default Desk/Viewport Touch, all five
        viewport classes, zero page scroll, and zero undersized targets.

pnpm.cmd --dir app run check:workspace-operator-viewport
  PASS: 1920x1080, 1920x1032, 2048x1152, 1366x768, and 1280x720.
```

The focused Auto VJ command remains a separate harness boundary: it stops at
`Program audio default-device config registration` before producing a layout
verdict. It was not counted as a product UI pass or used to close this
checkpoint. The broad legacy `check:viewport` runner also remains subject to
its known late CDP navigation timeout and is not used to manufacture a pass.

This checkpoint accepts current-source rendered containment and reachability
for the listed UI slices only. It does not claim native button-by-button live
Lighting/Video/Audio, Take/Transition, Blackout/Arm/Take Over, recording,
native accessibility, physical output, external clients, failure/recovery,
venue/soak, signing, publication, or product completion. Therefore
`UI-H5-CONTROL-001` remains `Open`.

## Current-source native rebuild and window probe — 2026-09-18

The exact current release executable was rebuilt after the short-height repair
with the maintained Tauri wrapper:

```text
pnpm.cmd --dir app tauri build --no-bundle
  PASS: exit 0; release optimized build completed.
  Toolchain: Build Tools MSVC 14.44.35207; where.exe link.exe resolved the
  pinned Hostx64/x64 linker first.
```

The rebuilt
C:\Users\janua\OneDrive\ドキュメント\GitHub\Syndocal\target\release\syndocal.exe
was then launched and checked as exactly one responsive Syndocal window
(PID 47928 at the time of this checkpoint).

The existing Windows native window/pane probe produced all product assertions
as PASS: maximized client 2560x1552, F11 2560x1600, exact Escape restore,
Control Lighting/Video/Both/Timeline semantic surfaces, both Stage/Timeline
detach orders, detached-pane restart restore, main-window reload adoption,
direct Stage-child close reintegration, and final full reintegration. The
fresh native screenshot set is under
C:\TEMP\syndocal-native-ui-short-height-20260918.

The wrapper process returned exit code 1 during its final Tauri-dev cleanup
despite the probe emitting every product assertion as PASS; therefore this
run is recorded as native probe evidence, not as an exit-0 native acceptance
claim or a Q4 accepted-current record. The two explicitly unverified native
pane-state boundaries remain those named in the generated report.

## Current DisplayConfig topology observation — 2026-09-18

The read-only three-display harness was rerun after the current host hardware
recheck. PnP still exposes multiple monitor and SMC-Mixer devices, but this
run's active DisplayConfig-to-GDI binding returned only one monitor identity:
`\\?\DISPLAY#TMA0803#5&2a56f61f&0&UID256#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}`
(`DISPLAY1`, `2560x1600`, effective DPI `192`). The harness therefore wrote
`verdict=not-configured`, `accepted=false`, and
`native_hardware_claim=false`. It made no display-mode, window, output, focus,
or device change. PnP device count is not substituted for the active
DisplayConfig identity contract, so `UI-H5-CONTROL-001` remains `Open`.

## Current exact-release PrintWindow visual recheck — 2026-09-18

After the native probe, the exact release executable was restarted and its
actual Syndocal HWND was captured with Win32 `PrintWindow`, rather than a
desktop screenshot that could capture a foreground browser. The process was
the current checkout's `target/release/syndocal.exe`, one responsive window,
client/window bounds `2586x1578`. The captured Control > Lighting > Live
surface showed the upper workspace, direct-control tabs, empty-state control
panes, lower Groups/Faders panes, and status bar fully contained; no upper
workspace collapse, clipping, or unreadable wrap was visible.

```text
artifact: C:\TEMP\syndocal-native-printwindow-20260918.png
sha256: 3EAEE18ED2069A9731F4C40DD33A07DDAD831B70CF009F844A269B914A8A37A1
PrintWindow: True
window: Syndocal / 2586x1578
```

This is one current exact-release visual slice only. It does not claim
button-by-button H5 live output, dangerous-operation, recording, recovery,
native accessibility, physical-display, external-client, or venue acceptance;
`UI-H5-CONTROL-001` remains `Open` for those boundaries.

## Capture helper foreground-isolation repair — 2026-09-18

The comparison capture helper was corrected after a false capture was found:
it selected the Syndocal HWND but used `CopyFromScreen`, so a foreground
browser could be saved under the target window's label. It now uses Win32
`PrintWindow(PW_RENDERFULLCONTENT)` for the resolved HWND and fails closed if
Windows cannot render that HWND.

The repaired helper was run against the exact release window. Its output was
the same SHA-256 as the direct HWND capture above, and visual inspection
showed Syndocal's Control > Lighting > Live surface rather than the foreground
browser:

```text
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File qa/harnesses/capture-window.ps1 -TitlePattern '^Syndocal$' -OutPath C:\TEMP\syndocal-capture-helper-fixed-20260918.png -SettleMilliseconds 300
captured 'Syndocal' (2586 x 1578)
sha256: 3EAEE18ED2069A9731F4C40DD33A07DDAD831B70CF009F844A269B914A8A37A1
```

This repairs evidence capture only; it does not expand the native H5,
accessibility, hardware, external-client, or venue acceptance boundary.

## Short-height Video workspace layout repair — 2026-09-21

The user-visible Video Control upper desk was too short on the current
200%-scaled desktop. The current maximized window measures `2586x1578` physical
pixels with a `2560x1552` client/work area, `192 DPI`, and an approximate CSS
client size of `1280x776`. The matching browser fixture is therefore
`2560x1552-2x` (`1280x776` CSS); the adjacent `2560x1600-2x` (`1280x800` CSS)
point remains for full-screen geometry. The layout gives scaled surfaces a
`60/40` upper/lower Video desk split and keeps Outputs and Layers side by side
in the lower context row. Compact low-DPI `720/768px` surfaces keep their
`48/52` split so the Media/Clip pane retains the existing minimum usable
height. No type, control, spacing, pad, or hit-target reduction was
introduced, and the CSS rule does not impose a physical-monitor role or
resolution requirement.

- Branch: `codex/showclock-review-20260912`
- Base before this checkpoint: `8c92fd223cb216f7e47a18578f0b538b7345d5e2`

The first `64/36` trial was rejected after `check:edit-ia-video` exposed a
real compact-layout regression: the Media/Clip pane fell below its required
`120px` at `1366x768` and `1280x720`. The split was corrected as above; the
same Edit gate then passed all five sizes. This is why the 200%-scale rule is
limited to the Video desk rather than applied to every short CSS viewport.

The exact `1280x776` Timeline probe also reproduced fractional clipping: the
last Phase action protruded about `1.4px` beyond its nested scroll region, and
the disclosure bottom was fractionally outside the workspace. The popup now
anchors `1px` closer and reserves `2px` of end scroll padding at short heights.
All 44px controls and hit targets remain unchanged; the nested Bank, Cue Audio,
and Phases reachability checks now pass at this exact viewport.

## Verification evidence

All listed commands returned exit code `0` on 2026-09-21. The browser fallback
used the installed Microsoft Edge executable because the in-app Browser
connector was unavailable.

| Area | Evidence |
| --- | --- |
| Control upper workspaces | `node app/scripts/run-control-upper-workspaces-browser.mjs` — PASS at `3840x2160`, `2560x1440`, `2560x1504`, `1920x1080`, `1280x720`, `2560x1504-2x` (`1280x752` CSS), `2560x1600-2x` (`1280x800` CSS), and `2560x1552-2x` (`1280x776` CSS, exact maximized client). Lighting, Video, Both, Timeline, expanded tools and mixer views completed; first Escape returned focus; every run reported zero runtime exceptions, console/log errors or warnings, and harness errors. Screenshots: `C:\TEMP\syndocal-control-ui-checkpoints`. |
| Edit Video / Media | `pnpm.cmd --dir app run check:edit-ia-video` — PASS at `1920x1080`, `1920x1032`, `2048x1152`, `1366x768`, and `1280x720`; one selected tab, usable media hit targets, zero document/app scroll. |
| Setup Video | `pnpm.cmd --dir app run check:video-setup-viewport` — PASS at the same five sizes; mapping remained reachable, `mapOverflow=0`, dock and preview contained, advanced controls reachable. |
| Frontend and native build | `pnpm.cmd --dir app tauri build --no-bundle` — PASS after `tsc --noEmit` and Vite (`359 modules`). MSVC Build Tools `14.44.35207` was pinned and verified as the first `where.exe link.exe` result. Vite emitted a chunk-size advisory for chunks over `500kB`; no TypeScript or Rust build failure occurred. |
| Exact release process | `target/release/syndocal.exe`, `66,216,960` bytes, SHA-256 `9C024EA8425D2831438C3282F5DE44EDE110529A6F1DE6E427F942720BD5503C`. Exactly one process for this executable path (PID `20176`), title `Syndocal`, non-zero HWND `0xF0DFC`, `Responding=True`; the window was maximized. |
| Native visual capture | `C:\\TEMP\\syndocal-ui-final-20260921.png` (`2586x1578`, SHA-256 `2BECEC66B667E09373DA6389BCC1DBF9430991D65ED18010FAD736B878EF66A2`), captured from the exact release window after the final build. The Video upper desk and lower Clips/Outputs/Layers panes are contained and readable. |

The native build consumed the current working tree, including unrelated
pre-existing user changes in `app/src/App.tsx`, `app/src/uiLocalization.ts`,
`app/src/remotePairingPin.ts`, and `app/scripts/check-dj-link-runtime.mjs`.
Those paths were preserved and are not part of this layout checkpoint; the
artifact hash above is therefore current-working-tree process/visual evidence,
not a claim that the artifact was built from the eventual layout-only commit.

This checkpoint confirms a rendered layout slice and a responsive native
window only. It does not validate native button-by-button operation,
accessibility, recording, Take/Blackout/Arm/Take Over, physical output,
external clients, or failure/recovery. `UI-H5-CONTROL-001` remains `Open`.

## All-screen UI recheck and standard-height VJ context reflow — 2026-09-21

The requested all-screen clipping review found one reproducible layout defect
in Fullscreen VJ at the standard `1920x1080` viewport: Outputs and Layers were
kept in two narrow columns even though the available lower context region had
enough height to give each pane a full-width row. The two-column rule had been
applied globally instead of only to the compact-height case. The current CSS
now stacks Outputs above Layers at ordinary heights and restores the existing
side-by-side layout only at CSS heights up to `800px`. Text, controls, spacing,
and hit targets were not reduced.

The repaired geometry passed at `1920x1080` (both panes `1036px` wide and
`113px` high, vertically stacked), while compact layouts remain side by side
at `1366x768` and `1280x720`. The exact regression gate was rerun after the
change:

```text
CHROME_PATH=C:\Users\janua\AppData\Local\Google\Chrome\Application\chrome.exe
pnpm.cmd --dir app run check:fullscreen-vj
  PASS: 1920x1080, 1366x768, 1280x720; no failed checks, critical telemetry
        overflow, or out-of-pane elements.
```

The wider current-source rendered UI pass covered:

| Surface | Result |
| --- | --- |
| Topbar / status containment | PASS at `1280x720`; status overflow `0`, required drag surfaces present, menu controls reachable. |
| Control Lighting, Video, Both, Timeline, tools and mixer | PASS at `3840x2160`, `2560x1440`, `2560x1504`, `1920x1080`, `1280x720`, plus 2x-scaled CSS `1280x752`, `1280x800`, and `1280x776`; focus return and diagnostic checks passed. |
| Setup I/O and Video | PASS at `1920x1080`, `1920x1032`, `2048x1152`, `1366x768`, `1280x720`; mapping/actions remained reachable and contained. |
| Edit Video / Media | PASS at the same five sizes; selected tab, media targets, and zero document/app scroll checks passed. |
| Touch | DVC preset 10 assertions and viewport matrix PASS at the same five sizes; no page scroll or undersized targets. |
| Workspace Operator and Live Audio | PASS at the same five sizes; Live Audio checked in English and Japanese with contained critical telemetry. |
| VJ Operator | English/Japanese compact and large viewport cases PASS; critical overflow and out-of-pane counts `0`. |
| Audio Reactive | PASS at `1920x1080` and `1366x768`; controls and SAFE ZERO strip contained. |
| Fullscreen VJ | PASS at standard and compact sizes listed above; normal-height full-width Output/Layer stacking and compact reflow both preserved. |

The in-app Browser connector was unavailable, so these rendered-browser checks
used the installed local browser through the repository's Playwright/CDP
harnesses. The legacy broad `check:viewport` harness did not mount its app
shell and is not counted as a pass; the focused screen-specific runners above
produced the stated results.

The current tree was then rebuilt with `pnpm.cmd --dir app tauri build
--no-bundle` using the pinned MSVC 14.44.35207 linker. The exact
`target/release/syndocal.exe` was relaunched as one responsive maximized
`Syndocal` window (PID `42176`). Its PrintWindow capture is
`C:\TEMP\syndocal-ui-review-codex-20260921\native-fixed-20260921.png`
(2586x1578, SHA-256
`541FF7570D815778B1E286CED95A7793DA64EF656F225FA38A385F7325BD8180`); that
native capture shows Setup > Lighting > Patch, not the Video screen. The
rendered Video fixture at the user's equivalent `1280x752` CSS viewport is
`C:\TEMP\syndocal-ui-review-codex-20260921\control-updated\control-video-1280x752.png`.
The release executable hash is
`95C8D76A7839ED8431C98F2ABDB9BDB4D48E48E73C4AF966143191A5D7FEC2FD`.
Because the build used the current working tree, its binary also includes
pre-existing user edits outside this layout checkpoint; it is not represented
as the output of the layout-only commit. Vite's existing chunk-size advisory
over `500kB` remains visible in build output.

No physical MIDI, DMX, video, or other output action was issued. This checkpoint
accepts rendered containment/reflow plus native build/window responsiveness
only. It does not close native button-by-button interaction, accessibility,
recording, Take/Blackout/Arm/Take Over, physical output, external clients,
failure/recovery, or venue acceptance. `UI-H5-CONTROL-001` remains `Open`.
