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
