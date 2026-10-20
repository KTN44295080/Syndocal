# Workspace information architecture: Prepare → Create → Perform

Base: `main`, `64bb36851983a66ef8f65592333f614533935ecb`. Task 15 is intentionally uncommitted for Claude's review. No version change or publication is part of this task.

## Rationale and navigation

The former Edit row mixed domains (Lighting, Video, Both) with activities (Timeline, Perform), and duplicated Control's Both overview. Setup prepares the show, Edit creates it, and Control operates it. Persisted workspace and mode IDs retain their existing meanings at the storage/API boundary; navigation adapters assign the live desks to their new workspace.

| Workspace | Visible tabs (EN / JA) | Internal route |
| --- | --- | --- |
| Setup | Lighting Patch / Video Outputs / I/O / Security; unchanged | `setup` |
| Edit | Lighting / Video / Timeline; 照明 / 映像 / タイムライン | `control` + `edit` / `mixer` / `live` |
| Control | Lighting / Video / Both; 照明 / 映像 / 両方 | `touch` + `edit` / `perform` / `both` |

F1/F2/F3 select Setup/Edit/Control. Alt+1/2/3 selects the current workspace's three tabs (Setup retains its existing area shortcuts). Existing E/M navigation is contextual; L retains Timeline navigation outside the Perform desk's clip-row key handling. Perform's row keys, arrows, Space/Enter, guarded Ctrl+Shift+B, and `?` remain owned by the desk. Arrow/Home/End keys on the domain tabs instead perform roving tab navigation.

## Old → new mapping

| Former surface | New destination and preservation |
| --- | --- |
| Edit > Lighting (`edit`) | Edit > Lighting; authoring surface unchanged. |
| Edit > Video (`mixer`) | Edit > Video; media preparation unchanged. |
| Edit > Timeline (`live`) | Edit > Timeline; existing authoring/playback surface retained. Global GO, Back, and blackout controls remain available. |
| Edit > Perform (`perform`) | Control > Video; the same Perform desk, with one upper matrix and lower Output/Inspector panes, ratios, Inspector tabs, FX audition, focus retention, and safety guards. |
| Edit > Both (`both`) | Control > Both, rendered once through `ControlBothPanel`. Open Lighting/Video now lead to Control domains. |
| Control `touchControlDomain=lighting` | Control > Lighting; existing editable touch surface and persistent stage/fader band retained. Domain tabs move to workspace chrome, so the toolbar no longer reserves overlapping tab space. |
| Control `touchControlDomain=video` | Control > Video > **Video live controls**. This modal retains the entire `TouchVideoPanel`: clip bank, import/queue/preview/Take, quantization/transition settings, compact transition buses, output selection/status, layer play/pause/Cue In/direction, opacity/speed/seek, enable/solo/loop, and add/jump cue points. It does not replace or resize Perform. |
| Control `touchControlDomain=both` | Control > Both. Its callbacks and readouts were compared with `ControlBothPanel`; the latter contains the same cue actions, masters, per-domain/all blackouts, monitor/output truth, and additional clip/recording information. The duplicate `TouchBothLivePanel` and its dedicated CSS were removed. The merged overview retains 48px live-control targets and internal scrolling. |

The tools modal owns its keyboard input and clears its floating inspector on close. Only Edit Video renders the independent floating inspector outside that modal; this prevents duplicate inspector instances and hidden dialogs from suspending Perform keys. Control touch pop-outs synchronize mode/domain and retain domain switching.

## Persistence and compatibility

`syndocal.workspaceLayout.v1` remains the storage key. `workspaceLayoutFromUnknown` performs an idempotent, one-way migration: former `control/perform` becomes `touch/perform` with `touch_control_domain=video`; former `control/both` becomes `touch/both` with domain `both`. Perform split ratios and all stable mode IDs are retained. The normalized layout is saved by the existing persistence effect. Named workspace profiles use this same parser.

A Setup layout with a dormant Perform/Both mode (the old Edit memory) is normalised to Edit Lighting with the equivalent remembered Control domain, so Edit never restores a removed desk. In session, returning from Control to Edit restores the last Edit creation tab (Timeline stays Timeline) through a non-persisted `lastEditMode`; leaving Control > Video for Setup and returning to Edit uses the same path.

The optional `touch_control_domain` field remembers the live domain while authoring. Old touch layouts without that field retain their former default Lighting surface. Invalid domain values normalize to that default; no unchecked route payload is restored. Pane IDs, MIDI/OSC target payloads, and split-ratio field names are unchanged. Pane initialization and private session navigation synchronize the owning workspace and effective mode.

New Project still confirms in the active Perform desk, including its relocated route. Blackout and Recover use the existing single authoritative path; this task changes their visibility guard, not the output authority or engine semantics.

## Evidence

Evidence is recorded under `target/qa/ia-15/` (git-ignored). Software-only browser evidence is not native or hardware evidence.

| Check | Result |
| --- | --- |
| `tsc --noEmit`, `pnpm --dir app build`, `check:localization` (4048/4048) | pass |
| `check:workspace-ia` (new; 12 tab/locale screenshots, module migration, boot migration, Partial-lock routes, Edit tab memory) | pass (5/5 consecutive after the dialog-close fix below) |
| `check:perform-layout`, `check:vj-operator`, `check:fullscreen-vj`, `check:workspace-header`, `check:topbar-pulse`, `check:live-desk-header`, `check:edit-live`, `check:touch` | pass |
| `check:project-shortcuts`, `check:destructive-actions`, `check:midi-learn`, `check:project-storage`, `check:no-browser-dialogs`, `check:hidden-child-console`, `check:frontend-command-routing`, `check:pane-window-lifecycle`, `check-fx-*`, `check-ui-theme`, `check-identity-color`, `check-desktop-window-mode`, `check-vj-media-import-access` | pass |
| `check-video-clip-slot-bank(-browser)`, `check-theme-browser` | pass |
| `check-viewport-containment.mjs` (broad) vs HEAD `64bb3685` baseline | zero new failing labels; 20 baseline-failing Control Mode labels now pass; see note |
| `check-control-upper-workspaces` (not a named gate) | Lighting, Video and Both phases pass after retargeting; stops at a Timeline overflow assertion at 3840x2160. HEAD cannot mount under this harness, so the HEAD result is unverified |
| `build-nobundle.cmd`, `native-gate.ps1` | pass (one responsive Syndocal window, exit 0) |

Broad viewport note: at HEAD the five `control-mixer-*` rows already failed the (stale) tab-count gate, which hid later assertions. With the gate retargeted they report `exclusiveUpperOwner`, `truthfulLibraryContent`, `operationalLibraryEntry`, `closedImportDisclosure` and `noDefaultDenseMixer` as failing. Every measured scalar in those rows is identical to HEAD apart from the tab counts, so the failures pre-exist (they encode an older Media-Library-only Edit > Video layout). Edit > Video was not changed by this task. They were stale and were re-targeted afterwards in STABILITY_FOLLOWUPS_2026-10-19.md.

Dialog close fix found by the new check: the Control Video live-tools dialog relied on its asynchronous `close` event to reap the nested clip inspector, which intermittently left an inspector mounted. The Close button now also clears the inspector synchronously.

Native walkthrough (real app, copy of the QA fixture, 1366x768 emulation; `target/qa/ia-15/native/`): navigated every Edit and Control tab; on Control > Video the row key `2` queued a clip, Enter took it (LIVE); the blackout dialog was cancelled (no change) and then accepted, Recover was driven by Ctrl+Shift+B and accepted; FX Browser Try auditioned on Preview only (Program unchanged, authoritative layer FX null) and Discard ended it; New Project showed the in-app confirmation and was cancelled; F2/F3 and Alt+1/2/3 routed as documented; Video live controls opened with the full TouchVideoPanel; the Touch pop-out pane window opened the Perform desk and was closed. The output role was restored to Standby and the six localStorage keys were restored byte-identically before closing. The fixture file hash is unchanged. Observed existing, unrelated messages: the fixture's "InvalidLayerTransition ... default clip slot" media-health text, "Monitor offline", and a project-publication sequence recovery-checkpoint error (also present in the Task 13b captures). The harness left a backend recovery entry for the QA copy in the project menu.

Independent source review caught and resolved touch-pane mode synchronization, tabpanel ARIA references, preserved 48px targets, domain-tab arrow handling, and nested inspector cleanup. Assertions are retargeted to the new route and single overview; the live-control size threshold is retained.

## Follow-up

Timeline authoring and playback were split in Task 21 (`qa/TIMELINE_AUTHOR_PLAYBACK_2026-10-20.md`): Control gained a fourth tab, Timeline (playback), and Edit > Timeline is authoring only. The text below records the state at Task 15. Timeline remained under Edit then; its transport and global safety controls remain available. Browser/native UI evidence in this task does not establish physical DMX/video, device, network, or venue acceptance.
