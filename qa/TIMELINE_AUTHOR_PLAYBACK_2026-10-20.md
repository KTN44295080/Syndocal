# Timeline: authoring (Edit) vs playback (Control), Task 21

Base: `main` `036a268c`. Worktree branch only; not merged, no version change, no publication. Browser evidence below is software-only: it is not native-window, DMX, video-output or hardware evidence, and Claude validates natively after merging.

## Decision: a fourth Control tab, not a fold into Both

Control now has four tabs: **Lighting / Video / Both / Timeline** (照明 / 映像 / 両方 / タイムライン). Edit keeps three: Lighting / Video / Timeline, and Edit > Timeline is now authoring only.

Why a new tab instead of folding playback into Control > Both:

- Control stays at four tabs, the stated ceiling.
- Both is a compact two-domain overview (Lighting cue/master card and Video monitor/master card). Adding a long cue list, a timeline transport and countdowns would either crowd that overview or push it into a scroll, and would make "Both" mean two different things.
- An operator running a show wants one place with GO, the cue list and the transport at large targets. That is a task ("run the cues"), not a domain mix, so it gets its own named tab next to the domains.
- Both is unchanged and keeps its own Back/GO/Release. Control > Timeline is the dedicated surface, and the top bar GO/Back/blackout stay everywhere.

## What each surface owns

| Surface | Contents |
| --- | --- |
| Edit > Timeline (workspace `control`, mode `live`) | Arranger (sequence, Scene Block and cue editing, lanes), the Timeline operator bar (transport used to audition while authoring, loop A/B, follow, metronome, Tools/Performance), Live Mixer disclosure, Timeline navigator, Sources shelf / Inspector / Video Preview drawers, and the desk surfaces Show / Automation / Playback (programmer, palettes, executors). |
| Control > Timeline (workspace `touch`, domain `timeline`, mode `live`) | Current and next cue with countdowns, Back / GO / Release / Pause Fade, Timeline position + transport (Play/Pause, return to start, seek), the cue list as 56px rows, DMX / VID / ALL blackout with the safety/standby state, and a shortcut legend. It sits above the same persistent stage/faders band as Both. |

Removed from Edit > Timeline: the hidden live status grid (active/next cue, counts, clock), the hidden Scene Matrix, Cue Pads and fade meter. They were rendered into `.liveControlPanel` and hidden by `display: none !important`; they were dead playback DOM and no longer exist. Lighting's own matrix/pads/status (`.editDomainUpperPanel`) are untouched. The Edit transport buttons are authoring tools (audition from the playhead), not the show-running surface; the top bar still has global Back/GO/blackout/Play in every workspace.

## Reuse and the single command path

`ControlTimelinePlaybackPanel` (`app/src/components/`) imports no backend module and calls no `invoke`; it only receives callbacks. `App.tsx` wires them to the same functions the top bar and keyboard use:

| Control | Command |
| --- | --- |
| GO / Back | `triggerNextCue` / `triggerPreviousCue` (the top bar's handlers) |
| Cue row | `triggerCue` |
| Release | `releaseCueById` (active cue) |
| Pause/Resume Fade | `setCueFadePaused` |
| Timeline Play/Pause/seek | `playTimeline` / `pauseTimeline` / `seekTimeline` (ticketed transport) |
| DMX / VID / ALL | `setBlackout` / `requestPerformVideoBlackout` / `setAllBlackout`, with the same Standby reason (`blackoutBlockedReason`) |

`check:workspace-ia` pins this wiring and the absence of any `invoke` in the panel. Countdown projections are pure read-only helpers (`app/src/controlTimelinePlayback.ts`, `check:control-timeline-playback`): the current cue counts down only while the engine's active fade names it; the next cue counts down only to an authored Timeline event at or after the playhead, otherwise it reads "Manual GO".

## Keyboard

- Alt+1..4 select the Control tabs, Alt+1..3 the Edit tabs (`workspaceModeForTabShortcut`; Alt+4 is unassigned in Edit). Tabs carry `aria-keyshortcuts`, the GO/Back/fade/transport/blackout buttons carry theirs, and the panel shows a visible legend.
- `L` in Control now selects Control > Timeline (like `E` selects Control Lighting); from Edit it still selects Edit > Timeline. `createAppKeyboardController` routes `live` to the domain when the workspace is Control.
- Space stays GO on the playback surface: `timelineSurfaceActive` (the Edit-only Space = play/pause, `L` = loop, `[`/`]`, Shift+A/B) now requires workspace `control`. `T` toggles the transport, `P` fade pause, `B`/`V` blackout, `1..0` cue pads, `G`/Enter GO, Shift+Space Back, exactly as before.
- As in Perform, Enter/Space on a focused button activates that button rather than firing the global GO, so Release or a cue row is never hijacked.
- Shortcut matrix check updated (`check-project-shortcuts`: 4 Control tabs, Alt+4 null in Edit, `L`/Space/Shift+Space/`T` in Control).

## Persisted ids and migration

No existing id changed meaning. `WorkspaceTab` (`setup | control | touch`) and `ControlMode` (`edit | live | mixer | both | perform`) are unchanged, as are `syndocal.workspaceLayout.v1`, pane window ids (`touch`, `timeline`, ...), split-ratio fields, MIDI/OSC target payloads and the Perform dispatcher. The new domain value is additive: `ControlDomain` gained `"timeline"` (`touch_control_domain`), mapped to mode `live` under workspace `touch`.

`workspaceLayoutFromUnknown` needs no data rewrite because no persisted surface moved:

- A saved Edit > Timeline layout (`control` + `live`, any `timeline_desk_surface`) still opens Edit > Timeline.
- A saved `touch` + `live` layout without a domain still falls to Lighting (existing rule, asserted in `check-workspace-ia`).
- `touch` + domain `timeline` restores Control > Timeline (mode normalised to `live`, idempotent). An unknown future domain fails closed to Lighting.
- A layout written by this build and opened by an older build falls back to Lighting through the old enum guard.
- Partial operator lock may use the playback surface (it routes `live` under `touch` as a permitted live desk); Edit Lighting stays locked.

Navigation history, F2/F3 and the in-session "last Edit tab" work across the two Timeline surfaces; the playback header has an **Edit Timeline** button (hidden in pane windows) and Edit's last tab is remembered through F3/F2.

## Styling and text

Theme tokens only (`app/src/components/ControlTimelinePlayback.css`), body text 13px or larger, tab/badge text in the chrome role only, 44px minimum targets (GO 64px, cue rows 56px). JA/EN strings added to `uiLocalization.ts` (`check:localization` 100%). No browser dialogs; blackout stays on the existing in-app path. The `theme` browser check now includes Control > Timeline in all eight themes at both locales.

## Evidence

Screenshots are in `target/qa/task21/` (git-ignored): `ia/` (14 tab screenshots, JA 1366x768 and EN 1920x1080), `control-timeline-cue-recall-*`, `edit-timeline-authoring-1920x1080-en.png`, `theme/` (96 images plus contact sheets).

| Check | Result |
| --- | --- |
| `tsc --noEmit`, `pnpm --dir app build`, `check:localization` (4112/4112), `check:project-shortcuts` (15552 cases), `check:control-timeline-playback` (new) | pass |
| `check:workspace-ia` (updated: 4 Control tabs, persistence/migration, playback structure and 44px targets, single command path, Alt+4 / L / Space, Partial lock, F2 return) | pass (consecutive runs; one cold-start mount timeout on the first launch after dependency re-optimisation, not reproduced) |
| `check:live-desk-header`, `check:edit-live`, `check:perform-layout`, `check:no-browser-dialogs`, `check:in-app-dialog`, `check:midi-learn` (controller + Perform routing), `check:workspace-operator`, pane-window lifecycle, frontend command routing, UI theme / identity colour, destructive actions, history keyboard, all 18 non-browser `check-timeline-*` scripts | pass |
| `check-theme-browser.mjs` (now includes Control > Timeline: 96 screenshots, 8 themes x 2 locales) | Captures, token contrast, rendered-text contrast and type roles pass after raising the legend and cue-fade text to the 14px control role (found by the check). The run still stops at the unchanged synthetic Perform state sampler (`[data-video-perform-cell-state="LIVE"]` never appears); the same step fails identically at HEAD |
| Broad `check-viewport-containment.mjs` (default `timeline` fixture, preview server) vs HEAD `036a268c` baseline | Zero new failing labels. Retargeted `hasExpectedControlModeSurface` for Control's four tabs (otherwise the five `control-both-*` labels would regress). Baseline and current fail the same categories (Scene Block large 3, persistent band 40, stage settings 5, timeline pane expansion 5, control mode surface 60, Scene Block context pane 5); Scene Settings fails 4 at baseline and 2 now (timing flake, not worse). Both runs end on the same pre-existing failures, which are stale assertions or environment issues |
| `check-control-upper-workspaces-browser.mjs` | Not a clean gate at HEAD here: the unmodified script hangs in the Edit > Video import-popup probe and at 3840x2160 (identical at HEAD). Its Lighting and Edit > Timeline phases (Video/Both skipped in a scratch copy) pass at 1280x752, 1280x720, 1920x1080 and 2560x1440 for both HEAD and this branch |

| `check-timeline-{inline-child,navigator,output-preview}-browser.mjs` | pass |
| `check-timeline-{authority,cue-audio,performance}-browser.mjs` | Fail identically at HEAD `036a268c` (authority: the same "Scene pointer drop on Audio Bed lane" IPC-ledger assertion in repeated runs; cue-audio: phase B startup IPC allowlist; performance: a timed-out wait), so they are baseline failures, not regressions |

Retargets made to existing checks: `check-localization` (one Cue Pad copy remains), `check-project-shortcuts` (Control Alt+1..4), `check-viewport-containment` (`hasExpectedControlModeSurface`), `check-theme-browser` (new workspace and a derived screenshot count).

## Not done / boundaries

- The Edit desk surface named "Playback" (programmer, reference palettes, playback executors) is authoring/setup of executors and was left in Edit > Timeline with its persisted id; its label may still read as show playback and could be renamed in a later naming pass.
- Live Mixer (group submaster/strobe/solo) remains a disclosure in the Edit > Timeline header; it was not part of the requested playback surface.
- `VideoPerformMatrix*` / `VideoPerformDesk*` and the Perform MIDI dispatcher were not edited.
- No native, DMX or video-output evidence; Claude's native pass after merge covers real GO/Back/Release, transport seeking and blackout with the real engine.
