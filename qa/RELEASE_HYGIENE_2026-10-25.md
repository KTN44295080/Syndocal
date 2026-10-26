# Release hygiene — admission before spawn and frontend chunks

Task 45d status: **pending; no staging, commit or push**. The native gate and
normal PID/port teardown are complete. Blackout/New Project now follow the
owner's immediate-action policy. The full official Perform CLI passes all six
viewport cases, then fails at another retired modal assertion for LIVE FX row
drops. Task45d requires other assertions unchanged; permission to update that
separate block was requested, and it remains unchanged. See Task 45d below.
Earlier observations and stop boundaries are retained as dated history.

Task: `task45-remote-admission-vite-chunks.md`. Shared checkout: `main`, base
`7b760b15eb71b9be5fa8e435ff2942ced49fca9b`. No staging, commits, push, version
change, distribution or physical output is authorized by this checkpoint.

The task's `app/src-tauri/src/remote_ws.rs` path is stale; the canonical file
cited by GAP-01 and changed here is `crates/io/src/remote_ws.rs`.

Evidence directory: `target/qa/release-hygiene-20261025/`.

## Scope and protected concurrent work

Changes are limited to Remote admission, frontend splitting, and this note.
The native-frame-attribution lane's Cargo manifests, `main.rs`, `crates/video`,
QA note, evidence and processes are preserved. `protected-before.json` records
the initial source hashes and the protected normal executable:
`232eb5c3a817475e93485ebcc7c9496a54a7c3746a7b847944f6a2c7bea35fe0`.
The existing run at `target/readback-copy-20261024/release/syndocal.exe`
(initial PID 24832) is also identified by that lane's `active-native.json` and
must finish before this task's native gate; its location differs from the
directory example in Task 45. No process belonging to it may be stopped.

## Baseline

The production frontend build completed with exit 0 and one Vite advisory.
`vite-before.log` records App 530.35 kB / gzip 164.94 kB and localization
369.70 kB / gzip 110.42 kB. The 500 kB warning threshold is unchanged.
Rust baseline compilation/listing used MSVC 14.44.35207, an absolute pinned
linker and verified PATH-first resolution; `remote-baseline.log` records zero
Rust compiler warnings. Listing tests is not counted as executing them.

## Remote admission

The listener acquires the existing RAII pending-head permit before assigning a
worker ID, cloning/registering a shutdown socket or spawning a worker. The
accepted peer address supplies the existing canonical per-IP identity. An
over-cap connection is counted and receives a best-effort nonblocking 503;
refusal never waits for the peer to read. The permit moves into the handler and
is released at the same head-processing boundary as before. Origin/Host,
pairing, grant, rate and size checks retain their existing code paths.

The new `pending_head_burst_is_refused_before_worker_spawn` regression holds
eight real workers before head processing, then starts 32 excess connections
simultaneously. It requires 32 HTTP 503 responses, peak live pending workers 8,
exactly eight assigned worker IDs, and empty worker/socket registries after
shutdown. A test-only per-server entry gate removes dependence on the one-second
HTTP deadline; it adds no production field or lock. Existing counter coverage
also proves the global 32 and per-IP 8 caps and permit reuse. This is a pending
admission bound proof, not a measured DoS or a blanket count of established
WebSocket/HTTP worker lifetimes.

| Check | Result | Actual exit |
| --- | --- | ---: |
| Baseline `cargo test -p io --lib remote_ws:: -- --list` | 75 listed; not executed | 0 |
| `cargo test -p io --lib pending_head_ -- --nocapture --test-threads=1` | 3 passed, 0 failed, 0 ignored, 257 filtered | 0 |
| `cargo test -p io --lib remote_ws:: -- --test-threads=1` | 75 passed, 0 failed, 1 ignored, 184 filtered | 0 |
| Tauri build-wrapper contract | 249 assertions, 27 hostile mutation fixtures | 0 |

The three focused tests are `admission_tests::pending_head_burst_is_refused_before_worker_spawn`,
`tests::pending_head_connections_are_capped` and
`tests::pending_head_slots_cap_each_address_and_the_total`, all under `remote_ws`.
The existing ignored test is
`remote_ws::tests::dj_link_operator_return_replacement_admits_real_rb_output_router_owner`;
it requires `SYNDOCAL_RB_OUTPUT_TEST_ROOT` and a local Node runtime.
No ignored physical/external-service test was enabled.

Rust compiler warnings for the measured io test configuration: baseline 0,
current 0, delta 0. Each valid Cargo invocation verified the exact Build Tools
MSVC 14.44.35207 linker pin and `where.exe link.exe` first result. The initial
bootstrap helper diagnostic was resolved with the official Visual Studio
Installer directory in PATH; it is separate from Rust compiler warnings.
Logs: `remote-baseline.log`, `remote-focused.log`, `remote-all.log`;
machine summary: `remote-summary.json`. Independent reviewer approved the final
source hashes and evidence in `review-remote.md`, with no blockers.

## Frontend splitting and startup

`App.tsx` retains controller, lifecycle and transport ownership. Cohesive eager
`AppWorkspaceView`, `PersistentWorkspaceView` and `SetupIoWorkspaceView` modules
own their existing JSX. Inferred typed models retain signal accessors and stable
callbacks. Explicit typed `invokeCommand` JSX props preserve the fail-closed
frontend command inventory; the generic callable is not hidden inside models.
The live-controls dialog ref/open/close lifecycle remains owned by App.

Only the transient Custom Profile workbench editor uses dynamic import. Its
nonpersisted `customWorkbenchOpen` starts false. Default and restored I/O first
screens remain eager; no startup screen waits for this import. `loadPanel`
limits loading to 15 seconds, clears its timer, and surfaces missing/rejected or
stalled chunks through a visible alert. The busy status and failure message have
Japanese translations. Back to Patch remains available in either state.
The Vite configuration and its warning threshold are unchanged.

| JS chunk | Before kB / gzip kB | After kB / gzip kB | Loading |
| --- | ---: | ---: | --- |
| App | 530.35 / 164.94 | 485.61 / 153.34 | Eager |
| AppWorkspaceView | Included in App | 61.33 / 17.78 | Eager |
| PersistentWorkspaceView | Included in App | 37.25 / 11.28 | Eager |
| SetupIoWorkspaceView | Included in App | 18.28 / 5.56 | Eager |
| CustomProfileEditorPanel | 6.37 / 2.21 | 6.37 / 2.21 | Eager before, dynamic after |
| LazyPanelBoundary | Absent | 0.83 / 0.47 | Eager |
| uiLocalization | 369.70 / 110.42 | 369.91 / 110.45 | Eager |

Vite warnings: before 1, after 0, delta -1; both production builds exit 0.
The aggregate of **all emitted JS assets**, including noninitial routes, is
3359.10 -> 3436.69 kB (+77.59 kB, +2.31%); gzip 1105.72 -> 1131.04 kB
(+25.32 kB). Chunk count is 424 -> 428. These totals are not initial-network
measurements. Typed view bridges add property names and allocate their model
objects once per App mount, outside frame/output loops. This change establishes
chunk boundaries and observed startup behavior, not faster startup, less total
JavaScript, lower memory, a throughput improvement or 60fps acceptance.

The independent review found no remaining source blocker. Expanded JSX matches
the original JSX apart from the intentional lazy boundary, explicit invoke
props and dialog callbacks. Canonical AST comparison found all 2,056 remaining
controller body statements identical to HEAD. Three model scans found no
captured mutable `let` variables; type-only model imports add no runtime cycle.
See `review-frontend.md` for identities, corrected intermediate findings and
the precise acceptance boundary.

| Check | Result | Actual exit |
| --- | --- | ---: |
| `pnpm --dir app exec tsc --noEmit` | Final clean verification, `typecheck-verified.log` | 0 |
| Production `pnpm --dir app build` | App 485.61 kB; no Vite warnings, `vite-final.log` | 0 |
| Async loader regressions | 5 passed, 0 failed/ignored, `lazy-panel-loader.log` | 0 |
| Production-browser lazy/startup proof | 4 scenarios passed, `browser-proof.json` | 0 |
| Existing workspace-operator, fixture-catalog, project-shortcuts, timeline-audio-output-bus-ui, scene-edit-faders, scene-edit-write-barrier | Passed; individual `check-*-after.log` logs | 0 each |
| Existing frontend Tauri inventory | Exact 513 commands, `check-frontend-tauri-invokes-final.log` | 0 |
| Existing Patch viewport checks | 12 scenarios passed, `viewport-patch-after.log` | 0 |
| Existing Setup I/O viewport check | 1 scenario passed, `viewport-io-after.log` | 0 |
| Five official App-only source-reader checks | Fail after JSX relocation; official scripts unchanged | 1 each |
| Source-aware copies of the same five checks | All assertions unchanged, 5 passed; Workspace IA also passes 14 tab/locale browser screenshots | 0 each |
| Existing Perform layout check | TAP button covered at 1920x1080; same actual failure on baseline HEAD App | 1 before/after |
| Existing frontend-command-routing check | Expected-count drift (511 versus manifest 513); baseline source has the same inconsistency | 1 |

The five App-only checkers are localization, workspace-ia,
output-control-runtime, timeline-source-shelf-contract and
timeline-cue-audio-runtime. Their proposed minimal source-reader migrations are
under `proposed-official/`, not applied to `app/scripts`. The task assigns source
ownership only to `app/src/**` and Vite configuration, so ownership extension was
requested and remains pending. Evidence-only `*-source-aware.mjs` copies retain
the original URL base and every assertion, aggregating App plus the three moved
views. Their passes prove those assertions over the moved code, **not** an
official CLI pass. Localization coverage is 4164/4164, with zero unprotected
bare user-data labels. Summaries: `frontend-focused-results.json` and
`frontend-source-aware-results.json`. Failed intermediate attempts are retained;
the clean root typecheck and final inventory/build/browser logs own final claims.

The Perform baseline is served through an evidence-only Vite loader of HEAD
App; no checkout/worktree/source reset was used. Both runs report the same TAP
failure at x=1619.75, y=86, width=40, height=40. This task does not fix unrelated
topbar geometry or change the command-routing check's expected inventory count.

Browser environment: Edge headless Chromium CDP, production preview at
`http://127.0.0.1:63905/`, 1920x1080. Browser plugin and Playwright package were
unavailable; the repository's existing CDP testing approach was used. Default
startup requested zero editor chunks. Opening Custom Profile requested one
chunk and rendered in 150 ms; input updates succeeded. Saved I/O startup mounted
eagerly and requested zero editor chunks. Injected missing-chunk failure showed
an alert in 129 ms; an intentionally stalled fetch first showed loading and then
an alert at 15095 ms. There were zero uncaught exceptions in these scenarios.
These timings describe this run only. Root and reviewer inspected the success
and timeout screenshots in `browser/`; surrounding shell and Back to Patch
remained available. `browser-proof.mjs` records the exact flow and assertions.

## Native gate and remaining acceptance

Native acceptance is **deferred, not passed**. Repeated process checks still
found the other lane's PID 24832 running from
`target/readback-copy-20261024/release/syndocal.exe`, identified by its own
`active-native.json`. No cargo/rustc belonging to it was interrupted, and no
Syndocal process was closed or terminated by this task. The user was asked to
close that lane normally after completion; no response/state change had arrived.
`native-wait-01.json` and `processes-recheck-*.json` retain the observations.

An isolated build config and guarded `build-native.ps1` are prepared under the
owned evidence directory. They target `native-target/` with an isolated app
identifier/profile and the pinned FFmpeg SDK. The preflight refuses any active
compiler/other-lane run and verifies protected hashes before invoking
`pnpm --dir app tauri build --no-bundle --config ...`. The maintained wrapper
owns exact MSVC initialization/pinning/PATH-first verification. The build was
not started while the other lane was active. The protected normal
`target/release/syndocal.exe` was not touched.

First safe resume: resolve the narrow checker ownership request and apply the
source-reader-only proposals if approved, rerun the official five checks, wait
for the other native lane to close, recheck processes/protected identities, then
run the prepared isolated build. Launch only its resulting executable, verify
exactly one responsive maximized Syndocal window, open the lazy editor, and
close normally. A changed source set requires current identity/evidence review.

No native build-warning measurement, native-window/lazy-editor acceptance,
physical device/network/DMX/audio/monitor proof, installer/package, release or
multi-machine/OS acceptance is claimed. OS clone/spawn failure injection and
live multi-address socket stress remain unverified. The pending-head bound and
browser checks do not substitute for those gates. This is an implementation
checkpoint with explicit unfinished acceptance, not completed release hygiene.

## Task 45b — official readers, inventory and Perform scope stop

Base remains `main` / `7b760b15eb71b9be5fa8e435ff2942ced49fca9b`.
Ownership was extended to the six affected scripts under `app/scripts`.
Evidence: `target/qa/release-hygiene-20261025/task45b/`. No stage, commit,
push, version change or physical output was performed.

Each of localization, workspace-ia, output-control-runtime,
timeline-source-shelf-contract and timeline-cue-audio-runtime now reads App
plus AppWorkspaceView, PersistentWorkspaceView and SetupIoWorkspaceView.
Each diff changes one source-read expression only. All 1,127 assertion call
expressions (281/91/440/212/103) are identical to HEAD, normalizing line endings
only. Both implementation evidence (`checkers/assertion-identity.json`) and an
independent stable-diff comparison (`layout-review/independent-assertion-review.json`)
confirm empty assertion deltas. Root inspected the actual diff and results.

The command-routing expected inventory was stale. Commit `2907a2ea` added
`query_video_authoring_lease_authority_v1` and `replace_video_authoring_lease_v1`:
the preceding manifest had 511 entries, the current one has 513, exactly two
additions and no removals. The query dispatch is at
`outputControlController.ts:1735`; replacement maps action to command at `:635`
and dispatches through the injected invoke at `:1493-1506`. Both appear in the
native `generate_handler!` at `main.rs:137344-137345` and have ReadOnly /
RuntimeMutation classifications at `control_plane.rs:422,519`. The checker
retains its exact inventory assertion with 513 and all mutation/routing
assertions. Product routing and the protected native sources are unchanged.

### Perform finding and explicit stop boundary

The unchanged official Perform CLI reproduces exit 1 at
`(x=1619.75, y=86, width=40, height=40)`. This is a closed-details visibility
mismatch, rather than two visible controls overlapping: Chromium retains the
TAP rectangle inside closed Tools, while `checkVisibility()` is false and
`elementFromPoint` reaches the underlying Media health region. Opening Tools
makes TAP visible and hit-testable at exactly the same authored size/position.
The diagnostic proves this for en/ja at 1920x1080, 1366x768 and 1280x720, with
zero console errors, correct page identity, meaningful content, no framework
overlay and no outer scroll. Root inspected the 1280x720 open-Tools screenshot.

Commit `69318204` intentionally moved BPM/Tap and sync into Tools and removed
`.topbarStateCluster`; `qa/UI_SINGLE_ROW_2026-10-22.md:22-25` records that shared
contract. `check-workspace-header-browser.mjs:158,168-174` requires Tools tempo
disclosure and one header row. Conversely Perform `:125-129,353-359` requires
visible tempo, sync nested inside the tempo cluster and the removed state
cluster. Opening Tools alone cannot satisfy those assertions. The current
shared-header checker passes at nine widths, including Japanese 110% and
focus/draft/resize/Escape interactions. Its displayed tempo targets retain
their existing sizes.

Satisfying the unchanged Perform assertions through product code would alter
the shared header and its current acceptance contract. Task 45b item 3 says
"If the fix turns out to require broad shared-layout changes, stop and report
instead." That boundary applies: no layout, typography, hit-target or official
Perform-checker edit was made. Resolving the contradictory acceptance contracts
requires a separately scoped decision. The diagnostic is not a Perform pass.

### Current validation

Browser plugin not available. Project Playwright CLI was unavailable; existing
official checks use their maintained CDP/Playwright paths without dependency
installation. Layout diagnostics used Edge headless CDP. Evidence remains
browser-only and does not establish native or hardware acceptance.

| Check | Actual exit | Result |
| --- | ---: | --- |
| TypeScript `pnpm --dir app exec tsc --noEmit` | 0 | No diagnostics |
| Production `pnpm --dir app build` | 0 | Vite warnings 0; App 485.61 kB / gzip 153.34 |
| Official localization | 0 | 4164/4164; unprotected bare user-data labels 0 |
| Official workspace-ia | 0 | 14 tab/locale screenshots and persistence/playback checks |
| Official output-control-runtime | 0 | Passed |
| Official timeline-source-shelf-contract | 0 | Passed |
| Official timeline-cue-audio-runtime | 0 | Passed |
| Official frontend-command-routing | 0 | Exact 513; 137 renderer / 31 server mutations; 28 raw / 505 facade dispatches |
| Loader regressions | 0 | 5 passed, 0 failed, 0 ignored |
| Official Perform layout | 1 | Closed-Tools TAP failure reproduced; explicit scope stop above |
| Official shared-header browser | 0 | Nine widths, current disclosure/size/focus contract passed |
| Six-case disclosure diagnostic | 0 | Hidden when closed; visible and hit-testable when opened; not Perform acceptance |
| Production VJ-bank viewport | 0 | Five viewports |
| Production Patch viewport | 0 | Five full viewports x four flows, seven responsive cases and scaling comparison |
| Production Setup I/O viewport | 0 | Five viewports |
| Assertion identity and independent stable-diff review | 0 each | No blocker; five reader assertion deltas empty |

The task45 Remote source/test SHA256s remain unchanged, so its conditional
Remote rerun is not triggered; the prior 3 focused and 75 Remote passes are
historical evidence, not executions in Task 45b. Rust warning baseline/current
for that earlier io test configuration remains 0/0 (delta 0). No Rust compiler
was invoked in this follow-up. Vite warning baseline 1 / current 0 / delta -1;
Task 45b itself starts and ends at 0. Chunk sizes and totals match the earlier
after table; no threshold or warning suppression changes were made.

### Task 45b native gate

At the end of frontend validation, protected PID 24832 still runs from
`target/readback-copy-20261024/release/syndocal.exe`. The guarded isolated build
is not started while it is active. Periodic bounded wait observations and the
final native outcome are recorded under `task45b/`. No other-lane process is
stopped or manipulated. All 11 protected source/runtime identities match the
original Task 45 snapshot, including the normal executable SHA256
`232eb5c3a817475e93485ebcc7c9496a54a7c3746a7b847944f6a2c7bea35fe0`.

## Task 45c — current evidence and pending boundary (2026-10-26)

Base: `main` / `e558fc5001f3f646d7b77d5042051e62777aeb3b`.
Evidence: `target/qa/release-hygiene-20261025/task45c/`.
Commits `7e618cca`, `f5cd3840` and `e558fc50` are preserved. Four historical
protected-source hashes changed through intervening committed work; this run
snapshots the current clean protected sources, not the obsolete Task 45 hashes.
All 11 current protected identities remain unchanged, including the normal exe.
There was no active other native/compiler lane at the isolated-build preflight.

The five official source readers retain exactly 1,127 identical assertion calls
(281/91/440/212/103), independently compared by TypeScript AST with current HEAD.
Only their source-read expressions change. Inventory remains exactly 513;
`query_video_authoring_lease_authority_v1` and `replace_video_authoring_lease_v1`
are wired through the output controller, native handler and admission classes.
Independent review approves the stable source diff, conditional on acceptance.

Perform assertion migration: **always-visible header TAP, sync nested in tempo,
and retired tempo/state/window ordering** -> **closed Tools hides TAP; keyboard
Enter opens Tools; the existing TAP is >=40x40, visible, fully unclipped,
unobstructed and reachable by Tab; Escape closes and restores trigger focus**.
Sync is a sibling in Tools. Current header controls retain pairwise overlap,
containment, one-row and overflow checks; disclosed tempo/sync cannot overlap.
Existing geometry, clipping-ancestor intersections and other control assertions
remain. `checkVisibility()` prevents stale closed-details rectangles passing.
This follows `69318204` / `UI_SINGLE_ROW_2026-10-22.md`, without product resizing.

| Current check | Actual exit | Result |
| --- | ---: | --- |
| TypeScript; production build | 0 each | App 485.61 kB / gzip 153.34; Vite warnings 0 |
| Five official readers | 0 each | Localization 4164/4164; IA 14 locale/tab screenshots; runtime contracts pass |
| Command routing; frontend invoke inventory | 0 each | Exact 513; 137/31 mutations; 28 raw / 505 facade dispatches |
| Tauri wrapper contract | 0 | 249 assertions / 27 hostile fixtures |
| Loader regressions | 0 | 5 passed, no failed/ignored |
| Production-browser startup/lazy/error proof | 0 | Four scenarios; no uncaught exceptions; saved I/O stays eager |
| Production Patch; Setup I/O; VJ bank viewports | 0 each | All five viewport sizes; Patch also responsive/scaled cases |
| Shared-header browser | 0 | Nine widths and interaction/containment checks |
| Tools-only diagnostic extracted from current checker | 0 | All six en/ja x 1920/1366/1280 cases; **partial proof only** |
| Official Perform layout | **1** | Tools/TAP passes; later retired Blackout-modal wait times out |
| Canonical destructive-action checker (independent reviewer) | 0 | Confirms current no-prompt live-action policy |
| Remote `pending_head_`; `remote_ws::` | 0 each | 3 passed / 257 filtered; 75 passed / 1 ignored / 184 filtered |
| Isolated `pnpm --dir app tauri build --no-bundle --config ...` | 0 | Exact MSVC 14.44.35207 pin/PATH-first; release build 6m36s |
| Owned `git diff --check` / additional-file whitespace | 0 | Final result recorded with file hashes |

Remote compiler warnings baseline/current/delta: 0/0/0. Vite: 1/0/-1.
Native release compiler warnings: historical baseline unmeasured / current 0;
no baseline delta is inferred. No suppression or chunk-threshold edits.
Initial viewport attempts used unsupported zero ports and failed; corrected
explicit ports passed. The first partial Tools diagnostic had a navigation
timeout under concurrent checks; its sequential six-case rerun passed. Original
failed logs remain. A stalled import's alert was observed after 34.080s wall
time during compilation load; its configured timer is 15s, not a hard scheduling
latency guarantee. This does not establish a startup/performance improvement.

**Additional Perform blocker:** owner decision `77d8c8b4` intentionally removed
Blackout and show-time New Project confirmations. The canonical
`check-destructive-actions.mjs` enforces that behavior, whereas Perform still
requires both retired dialogs. This is independent of the authorized TAP
decision. Product behavior and those action assertions are preserved; no fake
modal or deleted assertion is used to manufacture a pass. Exact history and
the next bounded checker task are in `review/perform-blackout-blocker.md`.

Native executable: `../native-target/release/syndocal.exe`, SHA256
`22edb4f73a02d57fcaa96b27eb628f854845a8670b1d0e43a03a117db67e8d08`.
It launched as PID **41760**, HWND **20451730**, exactly one responsive Syndocal
window, isolated identifier `jp.seraf.ktn.syndocal.qa.release-hygiene-20261025`.
Computer Use observed the real Patch first screen and two-row Patch header at
1280x800. Maximized state was not established: a maximize attempt was followed
by minimized-window errors. The plain-path recovery API unexpectedly launched
the installed `C:/Program Files/Syndocal/syndocal.exe`; it was normally closed
and process enumeration confirms it is gone. No installed-app output action was
requested. This detour is not native acceptance evidence.

Computer Use then stopped this turn because it could not confidently establish
the current Windows browser URL for policy enforcement. No further UI input
was issued. **Native lazy-editor, maximized Perform Tools/TAP, normal close and
PID/port teardown are pending.** `native-pending.json` records PID 41760 and its
WebView children; loopback listeners 63927 (WebView PID 53344) and 57750 (app)
remain. No process was force-terminated. First safe resume is to restore and
verify that exact isolated window, complete the native UI observations and close
normally, then prove the recorded processes/listeners gone. A blank recovery
browser tab is ephemeral and is not a deliverable.

**Stop:** Task 45c says to report pending when blocked. No files were staged,
committed or pushed, so commit-subject BOM/upstream equality checks do not apply
to a new checkpoint. After the separately scoped Perform action-contract update
and native completion, rerun the affected gate and inspect/stage only the 16
owned files before the authorized UTF-8-no-BOM commit/push. No physical output,
device/network acceptance, installer/distribution or release acceptance is claimed.

## Task 45d — native completion and remaining official-checker boundary

Shared `main`, resumed base `7a47f1409d0e012120f65da163d1aab67cd62de1`.
Evidence: `target/qa/release-hygiene-20261025/task45d/` and `task45d-resume/`.
Task45d authorizes the exact 16-file checkpoint commit after acceptance; its
instruction to keep other assertions unchanged remains the stopping boundary.
Concurrent `crates/video/**` work was neither edited nor staged.

The existing Task45d teardown record verifies original PID 41760, exact isolated
path and SHA256 `22edb4f73a02d57fcaa96b27eb628f854845a8670b1d0e43a03a117db67e8d08`,
restoration and normal app close, without forced termination. This resume
independently confirmed all seven recorded PIDs and ports 63927/57750 absent
before rebuilding (`leftover-recheck.json`).

**Action assertion changes:** owner decision
`77d8c8b4644f9256fa85ad175e9afdd7d3ee12ac`, “act at once on Undo-able removals
and live Perform actions,” removed confirmation for Blackout and show-time New
Project. Old: wait for a modal, inspect its text/focus, cancel/confirm it and
require another modal after Standby. New: Enter and Ctrl+Shift+B immediately
reach the canonical backend-required refusal in the browser preview, with no
in-app/browser dialog and unchanged authoritative blackout state; clean New
Project has the same immediate-dispatch proof. Each action first seeds a
distinct keyed Standby refusal and returns to driving, preventing a stale error
from satisfying the assertion. Existing Standby explanations, accessibility,
size, geometry and zero-document-scroll checks remain. Browser preview has no
backend; these assertions do not prove a successful native blackout toggle.

A three-line setup step scrolls the first Inspector parameter into view in its
existing internal scroll column before the byte-identical full-visibility
assertion. `VideoPerformQuickFx.css` explicitly owns that internal scrolling.
Complete ancestor clipping, hit testing, 44px size and zero-document-scroll
guards are unchanged. This proves reachability after scrolling, not initial
visibility. No product layout or sizing was changed.

The complete official CLI now passes en/ja × 1920×1080, 1366×768 and 1280×720,
plus both shared-bank focus cases. It then exits **1** at
`matrix-runtime-focus: a row drop asks for confirmation in the app`.
`VideoPerformMatrix.tsx` already adds LIVE FX immediately under the same owner
decision, but the later checker block still requires cancellation/confirmation.
That block is outside the named Blackout step and is preserved. The explicit
scope-extension question is pending; no fake modal, weakened assertion or
product behavior change was introduced. Independent read-only review verified
the policy, all 14 unchanged reviewed hashes, and the exact bounded checker
delta (`independent-review.md`). Earlier passing TypeScript, five official
readers (1,127 identical assertions), 513 routing/invoke inventory, loader,
production-browser, viewports and Remote results above are retained, not
relabelled as new executions.

**Native:** sources advanced since the old build, so the isolated target was
rebuilt with `pnpm --dir app tauri build --no-bundle --config ...`, verified
Build Tools MSVC 14.44.35207 absolute pin and PATH-first linker. Build exit **0**,
5m58s. The new isolated executable SHA256 is
`b16b2e88e16a289778c42627bad6a4b172b08b83746454f6ed2b8619e1ea6c6b`.
The final run was PID **63728**, HWND **529832**: EnumWindows found exactly one
visible `Syndocal` window owned by that PID; it responded and Windows IsZoomed
was true before UI actions. CDP listener ownership was verified against its
descendant tree. Native URL `http://tauri.localhost/`, 1280×752 CSS viewport.

The real Patch first screen rendered; its DMX header controls occupy one row
inside x=289..904.75, y=99..130.5. Custom Profile opened from Patch: parsed editor
modules changed 0→1, 16 inputs rendered, and no loading/error state remained.
Control → Video → Tools exposed a fully visible, unobstructed **40×40** TAP at
x=980.734375, y=85.5; Escape closed Tools. Screenshots were inspected. There were
no native console warnings/errors during this flow. This is not full profile
layout acceptance: the narrow existing workbench clips the Back button's top
by 2 CSS px and shows partial right-hand content; this observation is retained
without an unrelated layout change.

The initial evidence-only native harness assumed browser resource timings
would enumerate the editor module; WebView did not supply that entry. Its next
attempt added an unnecessary full-containment check on Back and detected the
2px clip above. Both failed attempts are retained. Final harness uses actual
Debugger script-parsed events for module loading and checks the requested
editor-opening/Tools flow; it does not assert complete workbench layout.
All three runs used the normal app close button and were reaped without force.
Final native close removed PID 63728 and all six recorded descendants, and
listeners **63927/50549** (`native-teardown.json`). Protected normal executable
SHA256 remains `232eb5c3a817475e93485ebcc7c9496a54a7c3746a7b847944f6a2c7bea35fe0`.

| Task45d resume check | Actual exit | Result |
| --- | ---: | --- |
| Official Perform CLI | **1** | Six viewport cases + two focus cases pass; retired FX-drop modal wait fails |
| Perform syntax; canonical destructive-actions | 0 each | Immediate-action policy intact |
| Isolated pinned Tauri native build | 0 | Rust warnings 0; Vite advisory 0; App 485.61 kB / gzip 153.34 |
| Native opening/Tools flow, final harness | 0 | One maximized responsive window; lazy editor and TAP observed |
| Normal close / descendant-PID / listener teardown | 0 | No recorded process or listener remains; no forced termination |
| `git diff --check` and owned added-file whitespace | 0 | No whitespace errors; Git CRLF notices are not compiler warnings |

Measured io-test compiler warnings remain historical baseline/current/delta
0/0/0. Native release historical baseline is unmeasured, Task45c/current are
0/0; no historical delta is inferred. Vite baseline/current/delta is 1/0/-1.
No suppression or warning-threshold change. No device, physical output,
distribution or release-acceptance claim.

**Stop / next action:** obtain authorization to replace the separate obsolete
LIVE FX drop-modal assertion with exact immediate destination/add-count/no-modal
proof, then rerun the official CLI. Only after it exits 0 may the 16 owned files
be staged, reviewed, committed with a UTF-8-no-BOM message and pushed. Current
stage remains empty; no checkpoint commit exists, so no commit-BOM or new-commit
upstream-equality claim is made.

Final status capture also contains another lane's dirty
`crates/video/src/native_gpu_output{,_shared,_shared_tests}.rs`. These appeared
after the native run and are preserved. Native evidence above applies to its
recorded build snapshot, not these later uncommitted video edits.

## Task45e: Perform check aligned with the no-confirmation contract

Owner decision `77d8c8b4` (2026-10-20) removed live-action confirmations. The
Perform layout checker (`app/scripts/check-perform-layout.mjs`) still waited
for the removed dialogs. Each stale step now asserts the current contract
(immediate dispatch, no in-app or browser dialog, resulting state; in the
backend-less browser preview, the backend-required rejection with unchanged
authoritative state). No other assertion was changed.

| Step | Old (before `77d8c8b4` contract) | New |
| --- | --- | --- |
| Blackout key, Enter | in-app danger dialog opens; focus trap, Cancel/Escape/Confirm cycle | reaches the desktop-required path immediately; no dialog; blackout state unchanged |
| Blackout, Ctrl+Shift+B | opens the same guarded dialog; Cancel preserves state | immediate desktop-required refusal; no dialog; state unchanged |
| New Project, Ctrl+N (clean) | in-app New Project dialog; Escape declines | immediate desktop-required refusal; no dialog; state unchanged (unsaved-data protection stays covered by `check:destructive-actions`) |
| Blackout after Standby returns to driving | key asks again; Cancel declines | key acts immediately: desktop-required refusal, no dialog |
| LIVE FX drop onto a layer row | confirmation naming the layer and LIVE; decline adds nothing, accept adds | one drop immediately adds the exact card once to the dropped row; no in-app or browser dialog |

Each action is preceded by a seeded distinct status (Standby refusal, then
Driving) so a stale backend-required message cannot satisfy the immediate
dispatch assertion. The DJ Link disarm/return confirmations are not exercised
by this checker.

Final Task45e runs: official Perform CLI exit 0 (6 viewport/locale cases and
5 focus cases); one earlier run in this session timed out at a cold
`workspace chrome` wait and passed on rerun with no source change (transient
Vite warm-up, recorded rather than hidden).

**Finding handed to the UI density tranche (not fixed here):** the native
Custom Profile editor in a narrow pane clips the Back control top by 2 px and
shows partial right-hand content (Task45d observation).
