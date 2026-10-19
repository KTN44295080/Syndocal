# Stability follow-ups (Task 16), 2026-10-19

Base: `main` at `11b93a28`, working tree uncommitted. Evidence lives under
`target/qa/task16/` (logs, native harness, screenshots).

## 1. The app stayed alive after "Stop and Close" (P1, fixed)

**Reproduction (release build, QA fixture copy, native).** The hang is not
about dirty projects, playing video, blackout, a pop-out pane or the Preview
audition on their own: those exited cleanly in every run (plain active
output x3, blackout x2, audition, an open in-app dialog). It needs the
machine output role to be changed to Standby while the managed Lighting+Video
lease is armed (Enable Output, then Role = Standby), and then a close. Both
earlier reports (Task 14, Task 15) set the role back to Standby before closing.
Before the fix that sequence failed on the first run, with the process alive
and no window (`native/evidence-before-hang-stderr.txt`, 141 threads, window
gone, `jp.seraf.ktn.syndocal-siw` alive):

```
Destroyed window managed output boundary was fail-closed: Managed automatic-boundary fail-stop did not authorize exact release
Process exit was blocked by managed output boundary: Managed output authority is already Faulted and remains retained
```

**Root cause.** Temporary logging of the fail-stop report (removed afterwards)
showed the single failing step: `RetireDmxUsbArtNet: Managed show DMX fail-stop
requires the exact Failed/Standby local Both failure fence epoch`
(`crates/engine/src/lib.rs`, `managed_show_dmx_fail_stop_authority_preflight`
requires `desired_role == Both`). The Standby role commands
(`set_output_ownership_role`, `arm_output_ownership_role`) applied Standby
without running the managed lease boundary that `start_standby_sync` already
runs. After Standby the lease was still `held_active` and the keepalive
manager still Armed, but the role no longer matched what its fail-stop needs.
The window-destroyed boundary then could not retire the lease, the manager
stayed Faulted, and the `ExitRequested` gate (`prevent_exit`) kept a process
with no window alive; the single-instance helper kept the app from relaunching.
There was no join, lock or IPC wait: the process was idle by design.

**Fix.** `app/src-tauri/src/main.rs:79332` `apply_standby_role_after_managed_boundary`
and `:79346` `apply_standby_output_ownership_role_command`; both Standby role
commands (`:79368`, `:79387`) now run the managed `Standby` boundary (fail-stop
plus exact release) first, outside every lifecycle/transition lock, then apply
Standby. Standby only denies output, so it is never withheld: if the managed
release cannot finish, Standby is still applied and the failure is reported
("Standby was applied, but the managed output lease could not be released: ...").
Tests: `standby_role_retires_the_managed_lease_before_the_role_changes` (order,
failed release still applies Standby, failed apply reported) and
`every_machine_role_standby_entry_point_uses_the_managed_boundary` (source
guard).

**Evidence after the fix** (final executable, sha256 in
`native/exe-sha256-final.txt`): Enable Output, Take, Role = Standby, dirty
project, close, "Discard, Stop and Close": 5 of 5 exits in 1-2 s with empty
stderr (`native/final-standby-runs.txt`), plus 5 of 5 on the previous build
(`native/after-standby-runs.txt`). After Standby the ownership is Standby/Ready
and the lease query reports no held lease (`native/after-standby-state.json`).
The audition variant (Preview audition active with the Both lease and a playing
layer) and the open-in-app-dialog variant (blackout dialog open when the close
is requested; both dialogs coexist) exit in 2 s (`native/after-variants.txt`).
The native gate passed (`logs/native-gate.log`).

**Limits.**
- Not exercised natively: a physical Display output window and recording.
  Opening the output window from Setup > Video > Outputs was rejected with
  `OutputControl rejected (stale_fence)` in this harness on every attempt, and
  recording needs an enabled output. Both are inert in the QA fixture, so the
  "active video output" and "recording" variants are not proven.
- The exit gate is unchanged: if a managed all-deny genuinely cannot finish at
  exit, the process is still kept alive without a window, by the original
  fail-closed design. This change removes the known cause; it does not add a
  recovery window for the unknown ones.
- The boundary runs before the transition lock is taken (as in
  `start_standby_sync`), so a concurrent arm between the two could still arm a
  lease after the boundary; the window is the same as the existing Standby sync
  path.
- A forced kill of the hung process was required during reproduction (exact
  path of this checkout only).

## 2. "Project publication request sequence must be 92, received 85" (P1, fixed)

**Root cause (not a stale-QA-only artifact).** The renderer keeps its next
request id and one unfinished request in `localStorage`
(`syndocal.projectPublication.v1`); the backend journal
(`project-recovery-authority.json`) is fsync-durable and is the authority. The
real profile held an unresolved autosave-backup intent `85` written by an older
renderer while the journal's high-water mark was 91. The harness caused this
one by restoring localStorage to an earlier snapshot, but a normal user can
reach the same state: browser storage is flushed lazily, so a crash, forced
exit or power loss right after an autosave leaves the journal ahead of the
renderer. The old code then failed forever: for a retired owner the resolver
answered "cannot prove this request was never reserved ... keep the durable
intent", and for the same owner the identical id was re-dispatched on every
autosave. Every 60 s the footer said "Recovery checkpoint failed: ..."
(`native/seq-baseline-footer.png`, copied to
`screens/recovery-message-before-fix.png`). The error is raised before the
journal is touched.

**Fix.**
- Backend `app/src-tauri/src/project_publication_missing.rs:27`: the
  missing-publication resolver answers `stale` (`reason` settled or unreserved,
  plus the journal's next id) when the origin has no pending/terminal entry and
  the id is not the next one. This is a proof: no unresolved entry, so every id
  up to the high-water mark was reserved and acknowledged and anything above it
  was never reserved. It also requires `acknowledged == high_water` (the
  validator only enforces `<=`), otherwise the intent is kept ("lags", `:131`).
  The journal is not written.
- Frontend `app/src/projectPublicationStorage.ts`: `resolveRetiredMissing`
  handles `stale` (`:756`); `retireRejectedSequence` (`:805`) handles a
  dispatch refused with the exact sequence error (or the reset-journal "must
  begin with request ID 1") once per call; `discardStaleProjectPublicationIntent`
  (`:1124`) retires exactly the stored intent (never while a terminal ACK is
  queued) and adopts the journal's id. A second refusal is a real fault and is
  reported. Nothing is dispatched under the stale id and nothing out of order is
  accepted.
- The operator sees: "Publication sequence re-synchronised: stored request N
  was retired; request M is next. No project data was changed. Save again if a
  save was in progress." (`App.tsx:16004`, Japanese in `uiLocalization.ts`).
- Changed acceptance: `missing_publication_rejects_unknown_old_shape_and_unresolved_origin_without_mutation`
  used to require an error for gaps and old ids; it now requires `stale` for
  them (journal unchanged) and still errors for unresolved origins, conflicting
  shapes and a lagging acknowledgement. `check-project-publication-e4.mjs`
  covers: both stale reasons through `start` and `recover`, malformed evidence,
  queued-ACK safety, the same-owner dispatch path, the bounded retry, errors that
  must never retire, the pinned backend message and the reset-journal case. Its
  controller window grew from 15 000 to 20 000 characters.

**Native check** (final executable): stored intent 85 with a retired owner, close
the app, relaunch: footer shows the message above (request 106 next) and the
stored intent is gone (`native/final-seq-result.json`,
`native/final-seq-resync-message.png`, `screens/recovery-message-after-fix.png`).
The earlier executable showed the same with request 100.

**Limits.** The renderer-side recovery for the "origin already has an unresolved
request" case (journal holds a pending request the renderer has forgotten) is
unchanged and still needs a manual abandon. The Settled/Unreserved proof depends
on the journal invariant that one request per origin is in flight at a time;
that is enforced at begin, not by the validator.

## 3. Control > Both polish (fixed)

- Runtime strings are localised: `None`, `No cue`, `No clip staged`,
  `No output selected` were passed inside `data-no-localize` elements; the panel
  now takes `null` and renders the fallback as ordinary text
  (`components/ControlBothPanel.tsx`, `App.tsx:27472`). The monitor status code
  `desktop-required` now uses the shared readable phrase
  (`liveVideoMonitorStatusLabel`, exported from `LiveVideoMonitorPanel.tsx`).
  `STOPPED` has a Japanese entry (`uiLocalization.ts`, next to `PLAYING`:
  `停止中` / `再生中`), so state words stay distinct (なし / キューなし for
  current / next cue).
- The overview uses `WorkspacePaneHeader` (title `Both`, context
  `Lighting + Video`, the two route buttons as tools); obsolete `.controlBothHeader*`
  CSS removed.
- Measured in the app (JA 1366x768): no text under 13px, all buttons 48px tall and
  14px; Back/GO/Release are dimmed only when they are really disabled (no cue).
  No button that looks disabled is enabled.
- Screenshots: `screens/control-both-1366x768-ja-tally.png`,
  `screens/control-both-1920x1080-en-tally.png` (Vite fixture),
  `screens/control-both-native-1366x768-ja-tally.png` (native, empty fixture).
- The `check-control-upper-workspaces-browser` selector for the two route buttons
  was re-pointed from `.controlBothHeaderActions button` to
  `[data-workspace-pane-header] [data-control-both-open]` (and likewise in
  `check-viewport-containment.mjs`): same two buttons, same assertion.

## 4. Stale `control-mixer-*` assertions (re-targeted, not weakened)

The five names are reported by the `--control-mode-surface-only` contract
(`hasExpectedControlModeSurface`; the broad run lists them but does not gate on them). Edit > Video has hosted the full Video
Control surface since `36f9d1a3` (UI-H1-REACHABILITY-001 removed `libraryOnly`
on purpose so Preview/Program, clips/Take, layers, outputs and Import Media are
reachable); `check-viewport-containment`'s fullscreen-VJ gate already requires
that dense mixer. The old assertions described the retired Media-Library-only
layout, so they were stale, not a UI regression. Decisions
(`check-viewport-containment.mjs`, `videoDomainChecks`):

| Assertion | Decision |
| --- | --- |
| `exclusiveUpperOwner` | Re-targeted: no edit-domain upper panel, no live panel, exactly one `.videoControlPanel` that is the mixer panel, plus the Edit preview and inspector panes. |
| `truthfulLibraryContent` | Re-targeted to the Media Library rail: rail count 1, summary count equals the number of items, empty state and a disabled Verify All exactly when there are none. |
| `operationalLibraryEntry` | Re-targeted: one Verify All in the rail and one visible Import Media entry (`data-vj-media-import-disclosure`). |
| `closedImportDisclosure` | Re-targeted to the mixer's disclosure: one disclosure, closed, one `data-vj-media-source-surface="mixer"`. |
| `noDefaultDenseMixer` | Inverted and renamed `denseMixerPresentOnce`: top pane, context pane, divider, monitors, master, outputs and layers each exactly once. |

With the re-targeting `control-mixer-1920x1080` passes the whole surface
contract (`logs/cms-3.log`). The other labels of that mode (`control-edit-*`,
`control-live-scene-settings-*`) already failed at HEAD for unrelated reasons and
were not in scope.

**Timeline overflow at 3840x2160** (`check-control-upper-workspaces-browser`):
`Timeline Bank/Cue Audio/Phases regions have no horizontal overflow` failed with
the Bank region 9 px too wide. It predates Task 15: it passes at `e6a65a73`
(before Task 14) and appears with `64bb3685` (Task 14 readability pass).
Cause: the global `input{width:100%}` stretched the "Follow to next Timeline"
checkbox and, with 14px nowrap label text, made the row wider than the popup
(it reproduced at 1920x1080 too, in English). Fix: `styles.css`
`.timelineFollowEditor .toggleRow` (wrap, checkbox `width:auto`). 3840x2160 and
1920x1080 now pass (`logs/upper-3840x2160-r5.log`, `logs/upper-1920x1080-r1.log`).
The checker also hits intermittent `Runtime.evaluate ... exceeded monotonic
cleanup deadline of 15000 ms` (requestAnimationFrame stalls in the headless
browser at 4K); this predates the change (also at `e6a65a73`) and is retried.

## 5. Remote web page dialogs (fixed)

`crates/io/src/remote_ws.rs`: the embedded page called `window.confirm` for
effect removal and `window.prompt` for naming a stored position. Both are now an
in-page dialog (`remoteAskDialog`, markup `#remoteDialog`, plain HTML/CSS):
`role=alertdialog` (confirmation) or `dialog` (name entry), Escape cancels,
Tab is trapped, the page behind is `inert`, focus is restored, a destructive
question starts on Cancel, IME composition Enter does not confirm. The remote
protocol is unchanged (`removeEffect` is still `{type:"removeEffect",effect_id}`).
Guards: `remote_page_asks_questions_in_page_never_with_browser_dialogs`
(detector self-test plus the page) and `check:no-browser-dialogs`, which now also
scans the embedded page (script AST plus a text scan for inline handlers; it
flags the previous page). Screenshots: `screens/remote-confirm-390x780.png`,
`screens/remote-store-position-390x780.png`.

## Validation

See the final report for the table. Windows rustfmt differs from Linux CI, so
only the touched hunks were formatted by hand.
