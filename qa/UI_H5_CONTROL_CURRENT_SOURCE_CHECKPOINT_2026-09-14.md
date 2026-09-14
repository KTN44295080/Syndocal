# UI-H5-CONTROL-001 current-source checkpoint — 2026-09-14

- Marker: `UI-H5-CONTROL-001`
- Branch: `codex/showclock-review-20260912`
- Base: `50f65a4cc7ae924f86ebd7177c1d27e2324b6bf4`
- Product code change: Control Both blackout toggles now expose `aria-pressed`
  and stable accessible action names; visible labels and authority callbacks
  are unchanged.

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
code `1`; no native interaction or physical output is claimed.

## Acceptance boundary

`UI-H5-CONTROL-001` remains `Open`. The existing evidence does not establish
native button-by-button interaction, full live Lighting/Video/Audio workflow
completion, native accessibility, physical output, failure/recovery rehearsal,
external clients, venue/soak, signing, publication, or product completion.
The native maximized/F11 gate also remains unverified until it is rerun on a
Windows operator display meeting the required `1920x1080` monitor boundary.

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
native build/process smoke and the recorded static checks. This does not turn
the unavailable rendered/native UI gates into a pass.

## Resume procedure

Run the H5 matrix on the exact current artifact: live Cue/Clip/Take/Transition,
Blackout/Arm/Take Over, recording and diagnostics, failure/recovery, and native
interaction at the required operator viewports. Preserve the first failure and
separate native, physical, and venue evidence before changing the marker.
