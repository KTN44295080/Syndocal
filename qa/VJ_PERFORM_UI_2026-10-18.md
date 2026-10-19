# VJ Perform desk — Task 11 finishing QA

Shared `main`, base `1c8e81b6`. Finishing changes remain uncommitted for Claude's
review and the owner's screenshot judgment. The matrix, desk and scoped Quick FX
implementation from Task 11 was already committed in `59fbfa7b`; this checkpoint
reviews that implementation and finishes its localization, state semantics,
show guards and acceptance coverage. No product version or native code changed.
PR #4 and its separate checkout were not modified.

## Design and review findings

- **Matrix:** 32 thumbnail columns per layer in a 100px-wide grid, with a sticky
  336px two-row layer strip (row 1: name, Solo, Bypass, Stop; row 2: symbol+word
  live state, opacity slider with inline percentage, blend), internal scrolling,
  selected outlines and text/symbol states. A clip cell is the trigger only
  (thumbnail, title, state, key badge). Per-clip actions live in the 44px sticky
  corner (Inspect and Clip... for the selected clip) and on right-click/long-press
  of a cell. Replaces detached four-clip/two-layer pages; addresses laptop visibility.
- **Monitors/transition:** compact Preview/Program beside the shared P button,
  Take, type/time/unit, master and guarded Video Blackout. At heights <=800px
  the monitor labels overlay the video and transition/master reflow into two
  rows (monitor row 128px, 104px at <=740px). Addresses hidden live
  controls and oversized monitors; reuses existing owners/ticketed commands.
- **Tempo:** header BPM, Tap, sync source/status and beat lights, using the current clock.
- **Quick FX:** existing selected-layer scoped ISF chain as one 66px row: a fixed
  260px "Quick FX / layer / Full FX editor" header, then each stage as a
  horizontal 44px row (number, name, Enabled/Bypassed, first two parameters
  inline). Addresses deep FX navigation and retains room on the right for
  Task 12; longer chains scroll horizontally within the strip.
- **Health:** a flexible, ellipsized region in the desk header between the tempo
  block and Keyboard shortcuts (full text in `title`). It consolidates/
  deduplicates monitor, staging, import, FX and media errors, with
  asset/unverified counts, Load Thumbnails and Retry monitors; it never covers
  the matrix and there is no separate bottom row.
- **Safety/keyboard:** row keys queue, arrows/Home/End select, Space/Enter Take,
  Ctrl+Shift+B confirms blackout, `?` opens the modal legend. Editing/Learn/modal
  suspend shortcuts. Perform New confirms even saved shows; remove-layer/clear-all
  are absent. Other desks retain their behavior.
- **Legibility/localization:** existing 14px desk text and >=44px targets retained;
  broken Japanese repaired, health/time labels translated, P button/key clarified.

References read before finishing: both operability walkthroughs and UI findings
in [the competitive review](COMPETITIVE_REVIEW_2026-10-16.md),
[Resolume layers](https://resolume.com/support/en/layers) and
[layouts](https://resolume.com/support/en/layouts),
[VDMX inspectors](https://docs.vidvox.net/vdmx/vdmx_inspectors), and
[SynapseRack layer controls](https://synapserack.com/en/docs/window/layer/).
The adopted convention is adjacent clips/layer controls and a prepared desk;
this is no cross-product usability or visual-parity claim.

## Intentional deviations and boundaries

- **MIDI:** BPM, Tap, master, blackout, opacity, solo and bypass use existing Learn.
  The model lacks clip-slot ID, Take, blend, transition, ISF stage/control and an
  equivalent Stop target. Requirement 6's full coverage remains unmet: extending
  native dispatch/persisted mappings violates the UI-only boundary, while fake
  metadata would route the wrong command. The legend discloses unsupported families.
- **Preview:** catalog Preview only authorizes thumbnails, so it is now Inspect.
  Actual Preview state derives from staged-layer/current-slot identity; live slots
  retain LIVE plus a Preview badge. P stages the current source. Arbitrary queued
  slot audition requires native transport work and remains unsupported.
- **Density:** at least three complete layer rows are visible at 1366x768 and
  1280x720 (five at 1920x1080), with >=44px targets and >=14px text kept.
  Vertical matrix scrolling still reaches further layers, so small screens do
  not imply all layers are simultaneously visible. See the density revision.
- **Per-clip actions:** the per-cell Inspect/More row was removed. Right-click or
  long-press on a cell selects it and opens clip options; the corner Inspect and
  Clip... buttons act on the selected clip (Inspect is disabled with no clip).
- Generated/live sources without catalog thumbnail data retain a source
  placeholder. Thumbnail inspection is not moving-video playback proof.
- Native captures use WebView2 CSS viewport emulation at DPR 1 while the real
  main window is maximized, as in the review. They do not establish physical
  Windows DPI/resizing, projector pixels, physical MIDI, DMX, external video
  peers or release/package acceptance. No CI wait, commit, push or `.agents/`
  staging is part of this task.

## Density revision (Claude review, 2026-10-18)

The first finishing pass measured only **one** complete layer row at 1366x768
(~164px rows, a 144px Quick FX panel and a 44px health row), which forced
vertical scrolling for every layer change on a laptop. The revision keeps all
type sizes (>=14px) and 44px targets and recovers space by reflow:

- Health moved into the desk header; the bottom health row is gone.
- Quick FX is a single 66px row (was 144px).
- The layer strip is two rows (~100px) and clip cells lose their action row and
  number badge, so a layer row is ~102px (was ~164px). The strip is 336px wide
  with the clip cells at 100px so that >=15 columns remain at 1920x1080. Row 1
  gives the layer name >=150px; the live state moved to row 2 as a leading
  symbol + word, and its Japanese words are 停止中 / 一時停止中 / LIVE, distinct
  from the Stop button (停止).
- At <=800px height the monitor row shrinks to 128px (104px at <=740px) with
  overlay labels and two-row transition/master controls.

Measured by `check:perform-layout` (browser fixture, 14 layers; complete rows
exclude the sticky head and the horizontal scrollbar):

| Viewport | Complete layer rows | Full clip columns |
| --- | --- | --- |
| 1920x1080 EN/JA | 5 | 15 |
| 1366x768 EN/JA | 3 | 10 |
| 1280x720 EN/JA | 3 | 9 |

The checker also asserts the 44px head, one button per clip cell, the strip row
structure, layer name width >=140px, the Quick FX row height, and the routing
of right-click and the corner Inspect/Clip... buttons. Browser captures in
`browser/` were regenerated for this revision. **The native screenshots and
native-run observations below predate the density revision and will be
re-captured**; their layouts (per-cell Inspect/More, bottom health rail, tall
Quick FX) no longer match the current UI. This is browser evidence only and
establishes no native-window behavior.

## Validation and evidence

Evidence root: `target/qa/vj-perform-ui/`. Browser plugin was unavailable; the
existing isolated Chromium CDP checker was used. Its real App cases test visible
controls and desktop-required refusal, while component fixtures test precise
callback routing/focus. These are separate from the actual release WebView.

| Check | Result and evidence |
| --- | --- |
| Frontend TypeScript + Vite | PASS. Initial `pnpm --dir app build` exit 0; both native builds also ran `tsc --noEmit && vite build`, exit 0. Final log `checks/native-build-final.log`. |
| Localization | PASS, 4013/4013 static strings, zero unprotected user labels; `checks/localization-final.log`. Japanese rendered checks additionally reject `??`/replacement-character corruption. |
| Perform layout | PASS, EN/JA x 1920x1080, 1366x768, 1280x720 (density revision: 15/10/9 fully visible clip columns and 5/3/3 complete layer rows; the original run recorded 15/9/9 columns and one row), required regions/hit testing/44px controls, Tab reachability, internal scroll, zero document scroll, keyboard routing, guarded blackout, modal legend and zero browser console errors/warnings. Four focus cases cover shared Perform/existing Control bank, matrix and Quick FX. `checks/perform-final.log`, `browser/results.json`. |
| Existing VJ operator | PASS, six EN/JA/size cases; `checks/vj-operator.log`. |
| Existing Fullscreen VJ | PASS, all three configured sizes; `checks/fullscreen-vj.log`. |
| Broad viewport | FAIL, exit 1. First cold source-server attempt did not mount (`checks/viewport.log`); built-preview retry completed and reported three scene-block-large and five VJ bank failures (`checks/viewport-retry.log`). All eight reproduce byte-for-byte with HEAD source overrides: `viewport-baseline/comparison.json`; baseline VJ/scene commands exit 1 and comparison exits 0. No acceptance assertions were weakened. These predate the finishing diff; broad acceptance remains failed. |
| Safety/shortcuts/workspaces | PASS: destructive-action execution tests (including saved-show cancel), project shortcut matrix 15552 cases plus 12 modal dispatch cases, workspace/operator policy 40 assertions. `checks/destructive.log`, `checks/shortcuts.log`, `checks/workspace.log`. |
| Existing routing/polling/MIDI/blackout contracts | PASS: frontend command routing, video runtime polling, MIDI Learn and safety blackout. `checks/routing.log`, `checks/polling.log`, `checks/midi-learn.log`, `checks/blackout-contract.log`. |
| MSVC/Tauri wrapper | PASS, 249 assertions/27 hostile fixtures; `checks/tauri-wrapper.log`. Exact Build Tools MSVC 14.44.35207 pin and PATH-first linker were printed/verified in both native build logs. |
| Windows release no-bundle build | PASS, both requested wrapper invocations exit 0. Final executable 72,121,856 bytes, SHA-256 `C0020A4DCB924212029145576A7D2C17E204105B40964EC43FAA73F21BE608BE`; final optimized build 3m40s. |
| Requested native gate | PASS, exit 0; `checks/native-gate.log`. One responsive maximized `Syndocal` main window, plus the 13x13 single-instance helper `jp.seraf.ktn.syndocal-siw`; normal gate close exited 0. The helper is not a second performance window. |
| Native performance actions | PASS for published UI/engine state: three layers, ten populated clips per layer, 11 catalog assets; mouse queue/Take, numeric-key queue, Enter Crossfade Take, opacity 1 -> 0.66, blend Normal -> Overlay, Posterize stage enabled -> bypassed, four Tap presses publishing 117.21256 BPM, native focused FX input retained across live updates, blackout confirmation declined then accepted, recovery accepted. `native-mouse-taken.json`, `native-queued-*.json`, `native-edited.json`, `native-transition.json`, `native-interactions.json`, `native-blackout-declined.json`, `native-blackout-*.json`, `native-recovered.json`. Monitor/pixel limitation below. |
| Final state/cleanup | Persisted/effective Standby, both output permissions false, blackout false (`native-final-standby.json`). DMX and all video destinations remained disabled during the test, including while the normal Enable Output UI issued the ownership lease (`native-owned-disabled-routes.json`). Original Recent list restored. Discard/Stop/Close removed the main window/WebView but left the single-instance helper process; exact owned PID/path cleanup ended it, with zero Syndocal processes remaining (`native-exit-result.json`). |
| Diff/reference check | `git diff --check` PASS. Independent finishing review completed; duplicate health messages found in review were deduplicated. Nothing staged or committed. |

Warning measurement: initial frontend baseline had one existing Vite chunk-size
advisory; final frontend/native builds have that same one advisory, delta zero.
Final Rust compiler warnings: zero. A separate native HEAD compiler baseline was
not run, so its baseline/delta is unmeasured. No warning thresholds or flags changed.

Native input used this executable's maximized 1280x752 CSS main window. The Windows
helper returned correct screenshots/accessibility but its pointer inputs resolved
to the 13px helper; reselect/retry reproduced it. Main-page mouse/keyboard input
therefore used the existing process-bound loopback WebView2 CDP session. Native
WebView confirmation dialogs were observed through Windows accessibility and
accepted with actual Enter; these were not replaced or mocked. Some initial
pointer/dialog CDP attempts timed out or hit guarded selector checks; their failure
was harness setup, and only subsequent published state is counted above.

The ignored QA show `vj-perform-final.sdc` uses the existing silent H.264 benchmark
and ten deterministic grid/number PNG fixtures in `media/`, with actual SHA-256,
byte sizes and default-slot source projections. An initial hand-authored fixture
with inconsistent default-slot projection was rejected without replacing the
project, then corrected before the test. No user show was overwritten.

## Owner screenshot index

The native PNGs below predate the density revision (see above) and will be
re-captured; the browser captures in `browser/` reflect the current layout.

All paths below start with `target/qa/vj-perform-ui/`; each PNG has a matching JSON
native authority/UI snapshot. The eight requested PNG dimensions and no-outer-scroll
metrics are checked by `verify-native-final.mjs`.

| State | 1366x768 | 1920x1080 |
| --- | --- | --- |
| Normal | `native-normal-1366x768.png` | `native-normal-1920x1080.png` |
| Playing + next queued | `native-queued-1366x768.png` | `native-queued-1920x1080.png` |
| Blackout | `native-blackout-1366x768.png` | `native-blackout-1920x1080.png` |
| Keyboard legend | `native-legend-1366x768.png` | `native-legend-1920x1080.png` |

English and Japanese before/after/legend browser captures for all three requested
sizes are in `browser/`. Native `native-transition.png` additionally captures the
real active Crossfade with a three-second duration and Reverse control.

## Native findings left unchanged

1. After taking a different asset from a layer's default slot, the monitor path
   returned `InvalidLayerTransition("Video layer 1 media asset projection does not
   match its default clip slot")`. The clip runtime published the new active slot
   and playback; the shared health region (then a bottom rail, now in the header)
   displayed the monitor error. See
   `native-edited.json/png`. Taking the default clip again restored the monitor;
   a later three-second Crossfade was observed in progress. The final QA does not
   claim rendered transition pixels or physical Program output: the configured
   Display was disabled throughout. This is a native observation, not repaired
   under the UI-only task. The initial invalid fixture rejection above is a
   separate, pre-operation fixture issue.
2. After the protected stop/close of the active test show, the main window and CDP
   listener disappeared but PID 6396 remained with only
   `jp.seraf.ktn.syndocal-siw`. Windows enumeration showed no targetable Syndocal
   window. The owned executable/path and PID were checked before terminating that
   remaining helper process. Clean startup/close in the requested native gate
   did exit 0; clean exit after this active workflow remains unverified.

Full MIDI target coverage, arbitrary queued-slot independent Preview, these native
findings and the eight baseline broad-viewport failures remain open. Claude review
and owner visual acceptance are next; no further roadmap item was started.

## Native re-capture after the density revision (2026-10-18, r2)

Evidence root `target/qa/vj-perform-ui/native-r2/`. Release no-bundle build exit 0
(73,019,392 bytes, SHA-256 `2485D00B40D2A099A4D9B4E54A9964E733AE87E2E02D05C3003C22C5671BEC05`,
zero Rust warnings, the same single Vite chunk advisory); native gate exit 0. All eight
owner screenshots were re-captured (`native-{normal,queued,blackout,legend}-{1366x768,1920x1080}.png`)
and `verify-r2.mjs` passed. Real-app interactions passed: mouse queue + Take, key `3` queue,
3-second Crossfade Take, opacity/blend edit, FX stage bypass, Tap (116.5 BPM), right-click
clip options without queueing, corner Clip... and Inspect, blackout declined/accepted/recovered.
Machine role, output permissions, fixture hash and Recent list were restored.

Open native findings (not repaired here):
1. Under the Standby role, Perform blackout is refused ("Exactly one active output lease ...
   required"); the refusal then stays in the status footer. The desk should explain or
   disable the control instead of failing after confirmation.
   Addressed 2026-10-20 (browser evidence only; native confirmation pending): the blackout
   keys (Perform desk, top-bar DMX/VID/ALL, Ctrl+Shift+B) read the output role and lease
   (`outputDriveStatus.ts`), stay focusable but show "Standby: this machine is not driving
   outputs", explain on activation without opening a confirmation, and the keyed footer
   note retires when the state changes. Emergency S0 and the runtime-latch release are
   unchanged; unknown state never blocks.
2. Clip Inspector: for a slot whose asset is Pattern 06, the Media asset select displayed
   asset 1, so Assign could reassign the wrong asset (`VideoClipSlotInspectorPanel.tsx`,
   outside this diff).
3. The known `InvalidLayerTransition ... default clip slot` monitor error and the leftover
   single-instance helper after an active-show close still reproduce.
