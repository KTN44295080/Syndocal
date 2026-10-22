# Single-row header and DMX Patch UI — 2026-10-22

## Scope and checkpoint

Base: `main` / `cfe8ca56a18873ebd0d967b74d73b337e6f31763`.
Campaign: `target/qa/ui-single-row-20261022/`.
The user prioritized this UI correction after the current FX performance capture.
The performance goal is still open; its safe closeout and unsuccessful A/B result
are recorded in [FX_DISPLAY_PERFORMANCE_2026-10-22.md](FX_DISPLAY_PERFORMANCE_2026-10-22.md).
The unaccepted borrowed-input Rust changes are outside this UI checkpoint.

Reference image: `target/qa/fx-perf-2026-10-22/native/ui-single-row-user-reference-20261022.png`,
2560×1504 physical pixels, SHA256
`ea9f467ab28d434b0978217e139862046b9c95ee93e8d0bf62ecc61b8e7fa734`.
The approved copy was viewed and its hash matched the supplied original hash.
The original Temp location was outside approved read roots; no permissions or
browser-extension settings were changed. The observed native viewport for this
display is 1280×752 CSS pixels at DPR 2.

## Resulting UI contract

- Navigation, project identity, Back/GO, DMX/VID/ALL, Tools, and window controls
  share one header row. The unconditional two-row layout below 1500px is removed.
- BPM, Tap, synchronization, and clock details share the existing Tools menu.
  One BPM component persists across window-size changes, retaining edit state.
- DMX Patch's heading, universe selector, and view operations share one row.
  At narrow pane widths, Grid/List and the address readout use the View disclosure.
  Usage and universe details use the same disclosure. Focus moves to the View
  trigger or the corresponding inline button when the placement changes.
- Next Free exists once beside the Source address. Its action selects the universe,
  returns to Grid, updates the address, and reveals the requested cell, including
  repeated requests after manual scrolling.
- Source, Grid, and Inspector remain three panes. No global font/control reduction
  or pane stacking was introduced. Source Custom/Refresh are locally represented
  by labelled icons; their measured boxes are 34×28px at 100% (their previous
  minimum height was 24px). This is not a claim that every hit-area width is unchanged.
- The canonical Patch authority, busy gates, fixture moves/cancellation, show
  output controls, and video failure/LastValid/cancel policies are unchanged.

## Focused checks and independent review

Artifacts retain separate started/exit JSON and full logs for each attempt.

| Check | Accepted attempt | Actual execution | Exit |
| --- | --- | --- | --- |
| TypeScript | `types-03` | `pnpm --dir app exec tsc --noEmit`; runner 46976 / child 10976 | 0 |
| Header | `header-06` | 9 widths (960–3840), normal and safe/dirty/locked states; Japanese 110% at 960/1280/1501; BPM draft/focus roundtrip 960→1280→960 and Escape cancellation | 0 |
| Patch viewport | `patch-05` | 5 full Patch viewports × populated grid / DnD / mocked GDTF / empty state; 7 responsive cases; scaling comparison | 0 |
| Pointer/selection | `drag-03` | Trusted pointer drag, stable cells, group offset across snapshot replacement, pointercancel, List bulk move, off-screen/repeated address reveal | 0 |
| Shared Control header | `control-02` | `2560x1504-2x` fixture, measured 1280×752 CSS; Lighting/Video/Timeline and Escape/focus checks | 0 |
| Localization | `localization-02` | 4162/4162 static Japanese entries; 0 unprotected bare user-data labels | 0 |
| MSVC wrapper | `msvc-01` | 249 assertions and 27 hostile mutation fixtures | 0 |

The sequential CoS session `58476` ran types/header/patch/drag02/control/localization.
Its overall exit was 1 because `drag-02` failed; later success must not overwrite
that session result. The corrected `drag-03` ran in session `8729`, exit 0.
The final Header/Control checks ran in session `78920`, exit 0.
No focused test is ignored. Browser checks use local fixtures/mocked backend
responses, not physical devices or a native backend. Control diagnostics reported
0 runtime exceptions, 0 console errors/warnings, 0 log errors/warnings, and 0
harness errors. TypeScript emitted no diagnostics. Compiler warning measurements
for the native configurations are recorded separately below.

One independent reviewer (`/root/review_borrowed_fx`) reviewed the stable product
and test diff, without edits, builds, process manipulation, or GUI actions.
Findings were resolved: P2 responsive recreation of focused BPM/View controls,
and P3 the untranslated selection tooltip. The reviewer read the corrected sources
and successful `header-04` / `types-03` evidence and reported no remaining source
blocker. The Patch focused-transition checks subsequently passed in English and
Japanese at 960px/110%. A subsequent root interaction audit reproduced another
P2: the Tools capture-phase Escape handler blurred and committed a BPM draft
before its input could cancel. The reviewer approved a guard scoped to the BPM
input inside that Tools instance. The input now owns its first Escape, while the
existing capture priority for other Tools and Timeline contexts remains intact.
`header-06` and `control-02` passed after the guard. This review does not substitute
for the native gate.

The same reviewer subsequently checked the final eight product hashes, Header/
Control logs, native geometry and action JSON, and the native QA helper. No product
blocker remained. The helper now asserts requested locale, scale, and actual window
width/maximized state, rather than merely recording them.

### Preserved failed attempts

- The original PowerShell check runner did not capture reliable process exit
  codes (`types-01`, `header-01`). These are invalid success evidence. The header
  failure log also identified the Learn prompt being obscured by the open Tools
  menu; choosing Learn now closes that menu and returns focus to its trigger.
- `patch-01`: bounded startup timeout; the separate diagnostic observed an
  11.589s cold Windows Vite mount, no runtime exception. The readiness allowance
  became 30s; geometry and interaction acceptance were retained.
- `patch-02` / `patch-03`: obsolete visible-usage/Refresh/English-aria expectations;
  retained scroll position in the small Overview required the test to scroll to
  the view button before asserting reachability. New tests assert usage inside
  the open disclosure and retain clipping/hit checks.
- `drag-01`: port 5173 was owned by the live Patch check; no concurrent retry or
  process kill. Subsequent browser checks ran sequentially.
- `drag-02`: new off-screen fixture did not constrain the actual map height;
  the address-reveal condition timed out. Constraining that fixture's map to 260px
  exercised real scrolling; `drag-03` passed without changing product behavior.
- `localization-01`: 4161/4162; corrected the new tooltip to an existing translation.
- `header-05`: actual draft 142 was committed instead of retaining 137.5 on Escape;
  session `65220`, exit 1. The same assertion passes in `header-06` after the guard.
- `build-qa-01`: Tauri CLI exit 2 before compilation because a campaign helper
  passed literal quote bytes in `--config`. CoS session `64216` / harness exit 1.
  Pin/PATH-first passed, source hashes stayed stable, no Rust compilation occurred.
  Failed log/JSON and `harness-build-qa-01.mjs` are preserved. A new attempt uses
  the validated whitespace/metacharacter-free campaign path without extra quotes.
- `build-qa-02`: compile succeeded, session `23825`, exit 0. Baseline video really
  compiled, 0 Rust warnings, 1 existing Vite large-chunk warning. While it was
  compiling, the new Escape test found the issue above; the build was followed
  to completion without restart or source edits. This executable was never
  launched and is superseded by the build containing the Escape correction.
  Its SHA256 is `bf7e9a0ae6f8ab092b6e7fda2579babd5f6d0ea780e276e1333b76a11969b59a`;
  binary/DLLs are in `runtime-qa-02`, UI sources and plan in `ui-source-qa-02`.
  `escape-source-refresh.json` records the sole product-source hash update.
- Initial `native-ui-max-en100` (CoS `36115`, exit 1) failed only its helper's
  `authoredSizing` assertion: it incorrectly required every header button to be
  40px high. HEAD CSS already specifies Project menu 29px and workspace tabs 34px.
  The helper now checks those unchanged sizes and 40px for primary/window controls;
  `max-en100-02` passed with identical measured boxes. No product sizing changed.
- CoS `19794` exited 1: the narrow-presentation helper's window API invocation
  failed before recording narrow geometry. Its subsequent Tools check exited 0
  at the previous window size; `tools-narrow-ja110` is **not** narrow acceptance.
  No Tauri permissions were expanded. The normal restore button and the exact
  QA HWND were used for the explicit native resize test.
- CoS `12482` exited 1: both attempts timed out connecting to the WebView debugger
  after interactive window sizing. PID 3224 stayed responsive and was not restarted.
  After Escape ended the window operation, the same debugger responded again.
  This observation does not establish the cause of the transient timeout. Later
  `narrow-ja110-03` and `tools-narrow-ja110-03` are the accepted narrow records.
- CoS `8969` exited 1 overall: process-tree capture failed because PowerShell's
  stdout encoding corrupted the Japanese executable path. The subsequent Standby
  command exited 0 and was not replayed. Explicit UTF-8 output fixed the read-only
  inventory; its next capture (`92301`) exited 0.

`cos-native-*.json` retain actual CoS exits and complete retained output, including
the failed sessions above. Failed attempts are not replaced by successful retries.

## Native gate and restoration

The final QA executable completed the native display/interaction gate and exited.
The standard-configuration build, deferred-source restoration and final integrity
check also completed successfully.

`native-plan.json` records the eight UI source hashes and the dedicated identifier
`jp.seraf.ktn.syndocal.qa.ui-single-row.1cd67575-0e9f-465e-ae11-05fd97332ea0`.
The normal identifier is `jp.seraf.ktn.syndocal` and its profile is not launched.
Preparation ran in CoS session `92688`, exit 0, with no exact app or compiler.
It verified the original runtime backup and current eight runtime files, saved
the deferred FX source, selected the already archived baseline Rust source, and
advanced its mtime so Cargo must actually recompile it.

Final QA build `qa-03` (CoS `42676`, exit 0; 05:25:55–05:30:36 UTC) used the
maintained `pnpm --dir app tauri build --no-bundle --config ...` wrapper.
MSVC 14.44.35207 was explicitly pinned and first in `where.exe link.exe`.
All eight UI hashes and baseline Rust stayed stable. QA02 had actually compiled
baseline video; QA03 reused that video artifact and rebuilt the application after
the Escape correction. The QA03 runtime is archived as `runtime-qa-03`.

Launch (`81606`, exit 0) verified exactly one responsive Syndocal window,
PID 3224 / HWND 4720888, executable SHA256
`b4e3751f5b1b990c2f9699948c1e5fb4eddcb65ce6bc9397f20ce9c4855419be`.
Bootstrap (`2070`, exit 0) bound the maintained native harness to this executable,
descriptor PID and loopback debugger process tree, maximized the actual window,
and observed 1280×752 CSS / DPR 2. Desktop screenshots were inspected alongside
the user's image. Trace and logging were OFF; stdout/stderr were empty.

### Actual native display and interaction checks

| Actual CSS viewport / DPR | Language / UI scale | Accepted geometry artifact | CoS / exit |
| --- | --- | --- | --- |
| 1280×752 / 2, maximized | English / 100% | `native-ui-max-en100-02.json` | `92625` / 0 |
| 1280×752 / 2, maximized | Japanese / 110% | `native-ui-max-ja110.json` | `92625` / 0 |
| 960×720 / 2, restored | Japanese / 110% | `native-ui-narrow-ja110-03.json` | `37456` / 0 |
| 960×720 / 2, restored | English / 100% | `native-ui-narrow-en100.json` | `12847` / 0 |
| 1280×752 / 2, maximized | English / 110% | `native-ui-max-en110.json` | `12847` / 0 |
| 1280×752 / 2, maximized, profile selected | English / 100% | `native-ui-max-en100-final.json` | `12847` / 0 |

Every record passed one-row header, one-row DMX heading/actions, Source heading
alignment, three-pane order, exactly one Next Free, 512 grid cells, usable grid
height, preserved authored sizes, center hit tests, ancestor clipping, overlapping
header items and outer horizontal overflow checks. Requested presentation conditions
are asserted in presentation mode. The restored window was physically resized to
1920×1440 client pixels at DPI 192 after matching PID/HWND/exe hash (`57239`, exit 0);
the later measured WebView viewport independently confirmed 960×720.

Native Tools tests passed at English 100% and Japanese 110% narrow: one BPM
component, reachable 40px/44px Tap, contained menu, draft 142 cancelled back to 119,
Tools kept open for the first Escape, then closed with focus returned to its
summary on the next Escape. The narrow Tools record follows the successful
`narrow-ja110-03` geometry on the same PID, with no intervening presentation change;
Tools JSON itself does not independently store viewport/locale/scale.

`native-ui-actions-narrow-ja110.json` (`50792`, exit 0) exercised real native
pointer/keyboard input at 960×720 / Japanese 110%. It selected the local verified
HSI 3-channel profile without patching a fixture, reached List through View,
and used the sole Source Next Free to restore Grid/address 1 with the cell visible.
Two further Ctrl+End-to-512/scroll cycles followed by Next Free also restored that
cell. The fixture array remained unchanged (zero fixtures), the project path stayed
the known QA fixture, and video/lighting remained disallowed in Ready/Standby.
The profile's existing authoring warning was visible; it is not a compiler warning
or a claim of physical fixture validation.

### Stop, exit, backups, and standard build

The explicit Standby call in `8969` exited 0 and recorded Ready/Standby,
video_allowed=false, lighting_allowed=false, no live/test output window, no active
external route and empty health. The QA-only unsaved edits were discarded through
the native Close dialog. Verification (`41898`, exit 0) found PID 3224 and all six
recorded WebView descendants gone, no exact app/compiler and no port 9272 listener.

Restoration (`96993`, exit 0) matched all eight runtime files to the untouched
original backup, including exe `cd4c3a1f…`, before the ordinary UI build. QA03 exe
and all seven DLLs remain archived. `original-runtime-restored.json` contains every
before/restored hash. Standard build `standard-01` completed in CoS session
`13095`, exit 0, 05:48:12–05:52:46 UTC, runner PID 38768 / child PID 38496,
log `build-standard-01.log`. It used the same eight UI hashes and baseline Rust,
without the QA config override, with MSVC pin/PATH-first and source stability
verified. Its exe SHA256 is
`232eb5c3a817475e93485ebcc7c9496a54a7c3746a7b847944f6a2c7bea35fe0`.
That normal executable and all seven DLLs are in `target/release` and archived
as `runtime-standard-01`; the ordinary user profile was not launched.

Deferred FX candidate source restoration (`69471`, exit 0) restored the original
candidate `lib.rs` and verified the unchanged borrowed-input test hash. Final
integrity (`78453`, exit 0; `native-final-integrity.json`) rechecked current and
archived standard runtime hashes, all original backups, all eight UI sources,
candidate/test hashes, unchanged ISF runtime/pass-runtime and build/config files,
and the known fixture. No exact app, compiler or QA listener remained. QA and
ordinary identifiers differ, while their main-window configurations are equivalent
(including the default false maximized setting). Native behavior was exercised
under the QA identifier; the ordinary identifier has compile/hash evidence only.

Warning counts for successful QA02, QA03 and standard01 were Rust 0 and the
existing Vite large-chunk warning 1, the same as the baseline native builds
(delta 0/0). No ignore or warning-suppression switch was added. Git's existing
LF-to-CRLF notices are separate from compiler diagnostics.

| Input | SHA256 |
| --- | --- |
| Original normal executable, also preserved in the FX campaign | `cd4c3a1f8d3048a9407d086de48d44be37c2711295bfd91f48038cd2cc12516c` |
| Final UI QA executable | `b4e3751f5b1b990c2f9699948c1e5fb4eddcb65ce6bc9397f20ce9c4855419be` |
| Final UI normal executable | `232eb5c3a817475e93485ebcc7c9496a54a7c3746a7b847944f6a2c7bea35fe0` |
| Baseline `crates/video/src/lib.rs` used for UI-only build | `f4b1aadd8b119b99cf1e3324487466f9ef70b8416db8d82ee498e8364a8b0ba6` |
| Deferred candidate `lib.rs` | `6a672620ecc25a06a72fbfe9f64a69091268c4409fa5c0cfd851138701a12430` |
| Deferred borrowed-input test | `029c77acd5a8a5997734ba5983b1f6f0801fb8ae180967730ada776c33d89ca3` |
| Known 3-layer / 3-stage silent H264 project | `1ad7bed8a2e59b6e86ae4a23800bb70e5f39554f9991dc9befb697ae9e11dd75` |
| Silent `h264_1080p30.mp4` | `53861258351df591b53450228d532c60fd914caa8de1d88b2029e480488e0c2b` |

## Boundaries

No speed improvement or 60fps acceptance is claimed. No unknown DMX/audio device,
external output, Mac gate, or ordinary user profile is exercised. The performance
candidate remains deferred with its evidence and source preserved. Only validated,
explicitly owned UI/QA files belong to this checkpoint; the restored candidate
Rust file and its untracked tests remain outside it. No new Display FPS result is
attributed to this UI change; its native gate exercised the main UI in Standby.
