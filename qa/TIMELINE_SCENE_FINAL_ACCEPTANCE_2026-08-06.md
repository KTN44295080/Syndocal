# Timeline and Scene Final Software Acceptance

- Date: 2026-08-06
- Platform exercised: Windows 11 native Tauri/WebView2 application
- Branch: `codex/syndocal-v1.0`
- Status: software acceptance PASS; physical-output and cross-platform boundaries remain below

## Scope

This acceptance closes the final software-side gap found while exercising Scene Matrix, direct Super Scene playback, child Timeline transport, native Daslight DVC import, and `.sdc` save/reload together. It does not claim total Daslight feature parity or replace a venue rehearsal.

## Representative show

The native import source was `C:\Users\kouty\Desktop\Shinkan-Left\Shinkan2026.dvc` (344,765 bytes). The import report recorded:

- 41 fixtures, 15 source profiles and 20 source fixture groups
- 16 scene banks and 77 Cues, including 2 Super Scenes
- 65 value payloads, 1,009 beam records and 0 beam mismatches
- 2 audio clips and 229 converted Scene Blocks
- 30 converted effects, 0 skipped effects and 0 unknown channel types
- 0 skipped fixtures/Cues and 0 missing audio files

## Defects closed

- Directly triggered Super Scenes now publish their live child Timeline position, duration, playing state and generation to the operator UI.
- The child Timeline Play/Pause, go-to-start and scrub controls now operate the direct runtime transport instead of remaining disabled at `0.000s`.
- Audio clips owned by a directly triggered child Timeline now follow that live child transport and its generation.
- DVC import no longer reports a fixture-group warning by treating `CueSummary.group_id` as a fixture target. That field is the Scene Matrix playback identity.
- Loading a replacement project clears direct child transport runtime even when the new project reuses the same Cue IDs. Authored/project state can reload, but position, playing state and generation do not leak across the project boundary.
- Runtime-only child transport publication is stripped from engine persistence, Tauri save data and frontend project comparison/storage.
- The three formerly skipped Chaser racks are now preserved as explicit source no-ops after verifying that all three original racks contain `BEAMS NB="0"`; no synthetic target or output is introduced.
- Scene Block `POSITION` is now signed end-to-end. The 29 positive source trims and 9 negative source pre-rolls in the representative show survive import, `.sdc` save/reload, live playback and seek/rebuild instead of clamping the negative values to zero.

## Native operator evidence

The debug Windows desktop application imported the representative DVC through the native file dialog. In Control, the `Shin` Super Scene exposed the `Show > Shin` child Timeline with a `5:05.064` duration and its audio strip.

- Trigger: the child position advanced from about `0.045s` to `7.795s` and the Stage preview changed.
- Pause: after pausing near `16.5s`, a later observation remained at about `16.608s` and reported `Timeline paused`.
- Resume: Play advanced the same transport to about `23.906s` and updated the Stage preview.
- Go to start: reset the position to about `0.114s` and reported `Timeline seek 0ms`.
- Scrub: a midpoint click moved the transport to about `2:41.584` and reported `Timeline seek 161583ms`.

The project was saved through native Save As to `target\qa\Shinkan2026-native-roundtrip.sdc`, then opened in a fresh application process. The saved JSON measured 1,594,922 bytes and contained 41 fixtures, 77 Cues, 1 Cue List, 17 embedded profiles and 33 fixture groups. The root Timeline correctly contained 0 events because the imported show is held by the two Super Scenes; the child Timelines retained their Scene Blocks and audio.

Fresh-process reload restored the authored show and live Cue state while the direct child transport began stopped at `0.000s`. A same-process reload after advancing the child beyond 16 seconds also republished `0.000s`, proving the project-load runtime boundary when Cue IDs are reused. Closing the cleanly reloaded project produced no unsaved-change prompt.

The signed-position follow-up was then repeated with the embedded release build. Its native import report showed 41 fixtures, 77 Cues, 229 Scene Blocks, 30 accounted effect racks and 0 skipped effects. The report displayed all negative source positions without clamping, including `-120`, `-3840`, `-40`, `-39`, `-400` and `-760 ms`. `Shin` entered LIVE, advanced its child transport and changed the Stage output. Save As produced `target\qa\Shinkan2026-native-signed-position.sdc` (1,596,018 bytes); a fresh application process reopened it with 41 fixtures, 77 Cues, 27 runtime effect targets and all 229 signed source-position fields intact: 29 positive, 9 negative and 191 zero. The nine negative values were `-3840, -760, -400, -400, -400, -400, -120, -40, -39`.

## Automated evidence

The final gate ladder was rerun from its first step after the last code/harness change:

- Rust formatting: PASS.
- Protocol/legacy serialization: 42 passed, including positive legacy values and signed negative source-position round-trip.
- Engine Timeline focus: 65 passed, including direct child audio and transport behavior.
- Engine Cue focus: 74 passed.
- Tauri project focus: 89 passed, including runtime stripping and Scene Matrix identity semantics.
- DVC importer focus: 30 passed, including the local full-Shinkan referential/save-reload test.
- Tauri Timeline audio focus: 6 passed.
- Frontend Timeline automation, live modifier, Cue effect recall, overlap, Timeline viewport, Scene settings/FX, Scene Matrix drag, Scene Live, project storage and localization gates: PASS. Static Japanese UI coverage was 2,809/2,809 with 0 unprotected user-data labels.
- Operation-count harness: 13/13 tasks passed. The three tasks with recorded Daslight observations were equal: Scene-to-Timeline placement 1 drag, layer Mute 1 click, Timeline pane expand/restore 2 clicks. Tasks without a recorded Daslight count remain unclaimed comparisons.
- Production frontend build: PASS; TypeScript and Vite completed. The main entry was 447.40 kB (134.78 kB gzip).
- Full viewport matrix: PASS at 1920x1080, 1920x1032, 2048x1152, 1366x768 and 1280x720 across Setup, Control, Touch, Scene Matrix, Timeline, Video/VJ, keyboard, project-menu and localization surfaces.
- Diff whitespace hygiene and background Vite/CDP port cleanup: PASS.

## Remaining acceptance boundaries

- No physical DMX/Art-Net fixture rig was connected for this pass, so electrical output, fixture response, venue timing and long-duration endurance remain a rehearsal gate.
- Audio ownership, timing and visible strip/runtime behavior were exercised, but audible output through a production audio device was not asserted.
- macOS and Linux packages were not run because those systems are unavailable. The shared Rust/TypeScript behavior is covered by automated tests, but native packaging and platform integration remain unverified there.
- The checks establish the implemented Timeline/Scene contract and representative-show usability. They do not establish that every Daslight feature exists or that the products are globally equivalent.
