# Syndocal completion flow — active dependency-ordered work authority

Date: 2026-08-19
Branch at creation: `codex/syndocal-v1.0`
Baseline before this document: `848d759846985cc3acf588356cfa3c996b4e2ef2`

## 2026-08-22 superseding local Windows show-core scope

The active completion target is now the operator's current Windows PC and the
show workflow below. It supersedes the broader Windows denominator for priority
and completion reporting:

- one Syndocal editor/control window plus two real fullscreen VJ display outputs,
  assigned to the LED-panel and projector roles;
- serial-port DMX lighting output and authored lighting scenes/banks at or above
  the pinned Daslight production workflow, including Static scenes;
- one authoritative Timeline arranging lighting scenes, real Media Library video
  files such as MP4, and audio-output clips;
- MIDI Time Code synchronization, DJ Link semantic synchronization through the
  completed DJ-PC Agent, MIDI-controller fader/control input, and audio input/output;
- the local separately built Windows ASIO path, including fail-closed driver,
  buffer, callback, fault, and restart behavior.

NDI and Spout are deferred from this target, not cancelled. macOS/Linux control and native proof,
distribution, signing, installer/updater publication, and legal/package disposition
are also deferred rather than removed. They remain recorded follow-on work and must not be described as complete,
but they do not block the local Windows show-core stop condition.

The former 14/71 percentage described the broader Windows product checklist and
is not the primary progress number for this narrowed target. Report show-core
software implementation, automated production-path proof, and current-PC physical
acceptance separately. Routine validation remains backend-first through the same
registered Tauri/control-plane paths; perform one final maximized native UI pass
only after the integrated backend bundle is green.

The operator confirmed on 2026-08-22 that a real lighting fixture and its serial
DMX interface are available for acceptance at any time. Lighting scene, Static,
Bank, FX, and Timeline authoring and the serial/Art-Net/sACN software output paths
are implementation-complete scope, not missing product features. Their remaining
stop gate is one batched current-PC physical run covering exact channel output,
44 Hz cadence, blackout/release, disconnect/reconnect, and bounded soak. It must
run through the production backend path before the single final UI pass.

### Edit workspace and unified Timeline UI contract

The Edit workspace has three persistent authoring domains: `Lighting`, `Video`,
and `Timeline`. They are product domains, not three independent playback engines:

- `Lighting` keeps the Bank-and-Scene matrix as its primary upper library;
- `Video` uses the Media Library as its primary upper library rather than replacing
  the whole Edit desk with a dense standalone mixer;
- `Timeline` exposes one named-timeline arranger with Lighting, Video, and Audio
  layers on the same time axis and playhead. It must never split those media types
  into separate timelines merely because their source libraries differ.

`Video` is the source-preparation domain, parallel to authoring a Lighting Scene.
It imports, verifies/relinks, organizes, and previews Media Library assets and
edits the reusable clip/source properties supported by the production model (for
example in/out, loop, fit/crop/aspect, audio use, effects, and authored routing).
It does not arrange show time. Physical display discovery, creation, calibration,
and projector mapping remain in Setup Video; Timeline alone owns temporal
placement and playback order. Advanced layer/Clip Slot/mixer tools remain
available by disclosure where they have authoritative routes, not as the default
Video workspace.

The Timeline domain keeps a co-visible source pane in the existing lower-right
desk position while the unified arranger occupies the upper pane. Its
`Scenes / Media Library` source switch presents the same authored Scene and
MediaAsset identities used by the full Lighting and Video domains; it is not a
second catalog or copied project state. Media may be filtered as All, Video, or
Audio. Operators drag upward from this pane directly to the visible target layer,
so no cross-tab drag or remembered hidden-tab gesture is required. The same
lower-right pane may switch to the selected-placement Inspector, but Sources is
the default Timeline authoring surface and remains directly reopenable and
internally scrollable at 1280x720.

The lower-left and lower-right Edit panes remain the shared desk skeleton. Their
contents are contextual: Lighting uses stage/selection plus fixture/scene
attributes, Video uses preview/output context plus selected media/clip properties,
and Timeline uses transport/preview plus the selected placement inspector. A Video
selection must not leave Lighting-only controls in the lower-right pane.

Media Library assets, including files containing both video and audio, are dragged
onto compatible Video or Audio layers. A single audiovisual file may create one
grouped Video plus Audio placement through one authoritative production mutation.
Authored Lighting scenes are dragged from their Bank column onto Lighting layers.
Every drag-and-drop route must retain a keyboard-accessible placement action and
must call the same registered Tauri/control-plane production operation as the
frontend pointer gesture. Wrong-kind layer drops fail closed with no project or
history delta.

### Timeline click and variable-meter contract

The current 25 ms polling metronome is not a completion-quality click engine. It
detects a changed quarter-note index after the fact, accents every fourth beat,
and emits a detached native sine tone. That implementation cannot prove bounded
jitter under load, exact MTC/DJ/loop behavior, or the 5/4 and 6/4 measures required
by the pinned song material.

The Windows show-core therefore requires one authored Timeline tempo/meter map
with an exact numerator and denominator per change point and a 4/4 default. The
same map drives arranger measure lines, snap, count-in, click accents, displayed
bar/beat position, MTC position conversion, and backend playback. The native
audio path schedules clicks against output sample frames with bounded lookahead;
UI timers and the 25 ms media-sync poll may request work but never define the
audible boundary. Stop, seek, loop, BPM/meter change, Timeline replacement, MTC
discontinuity, and DJ semantic transport rotation cancel stale queued frames by
an engine-owned generation before rescheduling.

The reference click tone is a 50 ms square wave: 1320 Hz on the measure downbeat,
920 Hz on other quarter-note beats, gain 0.1 with a 45 ms exponential decay to
0.0001. It has a separately controllable click bus and smooth gain change rather
than sharing the Program-media gain. Count-in uses the meter at the playback start
measure. Deterministic acceptance includes the `惑う星` meter sequence from
`C:\Users\kouty\Documents\Guiter\docs\click-engine-spec-ja.md`: measures 113-128
at 194 BPM must produce exactly 74 clicks with downbeats at the authored measure
boundaries. Full 7/8 and 3/8 handling is included by the numerator/denominator
model rather than narrowing the product schema to only n/4 meters.

### Timeline Guide voice contract

The existing Guide path is a retained foundation, not the completed Guide voice
surface. The engine already publishes generation-fenced `Phase`, `Looping`,
`Break`, and `Trans` cues; the native runtime plays the bundled deterministic
English WAV vocabulary through an independent session-local device/gain bus and
reports a missing requested device or unsupported custom label without silently
falling back. Browser-only fixtures may use Web Speech, but the Tauri product must
not speak the same cue twice.

The fixed embedded English vocabulary is the intended show scope. Custom Phase
labels remain visible text-only Guide faults and do not require TTS, a Japanese
voice pack, or a network service. Guide events must still be scheduled against
the same output-sample-frame and authored
tempo/meter authority as the click engine rather than appended after a 25 ms poll.
Loop and Follow-transition cadence must use the active numerator/denominator map,
not a fixed four-beat bar. Stop, seek, loop wrap, Timeline/project replacement,
Follow settlement, MTC/DJ discontinuity, and output-device restart cancel stale
queued speech by generation, while ordinary continuous MTC/DJ phase correction
does not truncate an already valid utterance.

Successful Follow settlement emits the fixed one-word guide `Complete` exactly
once. It is not emitted at admission, visual transition end, abort, Hold/Fault,
failed quorum, or stale-generation cleanup. At a settlement boundary where a
destination Phase would otherwise be due, `Complete` is the only Guide cue;
the destination Phase/Intro is suppressed so the two words cannot collide.

Guide enable remains project-authored. Physical device selection and monitor gain
are machine-local settings, must survive an application restart, and must stay
locked with a visible fault when the explicitly selected device is absent. The
Guide bus remains separate from Program audio and the Click bus. Playback captures
the Timeline BPM at each cue boundary and applies the natural rate adjustment
`playback_rate_milli = clamp(round((1 + (BPM - 170) / 600) * 1000), 920, 1080)`
to the fixed voice: 170 BPM is 1000 and 194 BPM is 1040. This is intentionally
not pitch-preserving; a later BPM or sync correction cannot retime a word that is
already in progress. Deterministic backend proof must cover every built-in
English asset, unsupported/corrupt custom
labels, exact ordering when multiple boundaries are crossed,
stale-generation cancellation, missing-device fail-closed behavior, and parity
between the registered UI command path and the backend production driver.

### 2026-08-22 pinned two-song click and Guide material

The operator-confirmed show chart, rather than the currently truncated FRET STEP
`totalMeasures: 151`, is authoritative for this Syndocal material. `人生オーバー`
has 156 authored 4/4 measures at 170 BPM. Its Guide chart announces Intro,
Verse, Pre Chorus, Chorus, Interlude, Breakdown, and Outro; each phrase begins on
the preceding measure's final beat, except the first Intro at frame zero. Measure
98 is one authored A-B measure repeated indefinitely by runtime transport until
F13 releases it. The source Guide has one `Looping` entry at measure 98; transport
rewind replays the same entry on every held pass without a counter. `Bridge` is
deliberately suppressed so it cannot overlap the loop call, and measure 99
announces `Break` only after release lets transport cross the boundary. Measures
149-156 form one
phase-continuous 32-quarter-note tempo ramp from 170 to 194 BPM. `Trans` is
announced every two measures, targeting measures 149, 151, 153, and 155 from each
preceding beat. The songs do not overlap; `Complete` begins on the final beat of
`人生オーバー` and its uncut tail crosses the exact boundary where `惑う星`
measure 1 begins at 194 BPM. The conflicting `惑う星` Intro call is deliberately
suppressed. `惑う星` announces its remaining Verse, Pre Chorus, Chorus,
Interlude, and Outro sections from the preceding beat and uses its confirmed
207-measure map with 5/4 at measures 117-119 and 121-123, 6/4 at 18, 120, and
124, and 4/4 elsewhere.

The deterministic pre-render tool is
`tools/audio/export-jinsei-madow-click-guide.mjs`; the matching Zira voice
generator is `tools/audio/generate-guide-complete.ps1`, and the product source
asset is `app/src-tauri/assets/timeline-guide/en/complete.wav`. Canonical
audition outputs live outside Git at
`C:\TEMP\syndocal-show-audio-indefinite-loop`: nine
48 kHz, PCM16, mono WAVs (per-song and connected click/Guide/mix stems) plus a
sample-frame manifest. The 2026-08-27 clean-break export contains 624 natural
`人生オーバー` source clicks, 840 `惑う星` clicks, 1,464 total clicks, and
26 physical/semantic source Guide events. Twenty-five Guide onsets are exactly
one beat before their target and the sole exception is the frame-zero
`人生オーバー` Intro. The connected boundary is frame 10,536,286; duration
is 23,006,389 frames / 479.299770833 seconds. Two independent perceptual exports
were byte-identical, all nine default WAVs reproduced against a fresh baseline,
all full PCM voices remained non-overlapping, and no output clipped. Manifest
schema v3 removes the finite-pass and synthetic-added-beat fields and records
exact `{98,99,indefinite,F13,automaticRelease:false}` runtime-loop authority.
These files prove the
pinned authored material and provide audition/backstop stems; they do not replace
the required runtime sample-frame click/Guide scheduler for DJ/MTC seek, loop,
tempo, or discontinuity handling.

## 2026-08-22 broader Windows-only reference scope

The current completion target is this operator's Windows PC, not a public
cross-platform distribution. macOS/Linux control, native acceptance, and warning
rows, plus distribution/legal/signing/notarization/SBOM/clean-machine/updater/
publication work, are deferred outside the active denominator. They remain useful
future roadmap material but do not block the current Windows completion claim.

This scope change does not weaken Windows requirements. Native release builds,
zero first-party warnings, the final three-screen VJ topology, Windows ASIO, DMX/MIDI/OSC,
DJ Link/Pedal integration, crash/reply-loss recovery, accessibility, security,
hardware fault tests, and one-hour Windows soak remain mandatory. The active
checklist denominator is 71 rather than 79; after E1 it is 14/71 (19.7%).

### Windows verification execution rule

Routine validation must drive the same registered Tauri/control-plane commands
as the frontend from backend production-path drivers. Repeated Computer Use or
manual UI passes are prohibited during individual implementation loops. Bundle
implementation, adversarial review, warning/build gates, and documentation into
stable checkpoints, and reserve one maximized release-executable UI/hardware pass
for the final integrated acceptance of the bundle. Backend drivers do not replace
that final native pass; they remove redundant intermediate UI operation.

## 2026-08-21 superseding OutputControl decision

The physical-input consent design is retired in full. Any older section or
roadmap item that requires a six-digit code, Raw Input keyboard challenge,
physical Enter, a 15-second challenge, or prepare/status/consume consent IPC is
historical and must not be reimplemented. The replacement contract is:

- normal local Output Enable is one explicit click and atomically acquires plus
  arms exact Lighting and Video authority;
- Release, advanced Arm, Take Over, Add Display, and Force Transfer require a
  parented OS-native Warning/Yes-No dialog, with Yes as the only accepting result;
- cancel/close is a retained terminal rejection, and retries do not reprompt or
  mutate output;
- all mutating OutputControl operations are local-only v2 commands with command
  schema 2 and registry wire schema 4; v1, unknown future versions, Remote,
  MIDI, OSC, DMX, Web Remote, shortcuts, and global hotkeys fail closed;
- DJ-PC Pedal/global-hotkey ownership remains solely in the DJ Agent topology and
  is not recreated inside Syndocal.

This decision is a product simplification and a compatibility boundary, not a
safety bypass. Owner incarnation, project/output/safety fences, durable terminal
receipts, exact replay, final revalidation, and S0 Blackout remain mandatory.

## 1. Purpose and authority

The operator rescinded the requested post-`1.2.0-alpha.10` pause on 2026-08-25
before its final authority promotion. This file is again the active
dependency-ordered work authority from the earlier AI3 checkpoint to a truthful
public release. `qa/SYNDOCAL_POST_ALPHA10_PAUSE_HANDOFF_2026-08-24.md` remains an
exact alpha.10 evidence, cleanup, frozen-worktree, and residual snapshot; it is not
a current stop. The following documents remain normative within their domains:

- `qa/CODEX_COMPLETION_ROADMAP_2026-08-13.md`: complete product requirements,
  traceability, severity, hardware, distribution, and final integrated gate;
- `qa/SYNDOCAL_AI_CONTROL_PLANE_ROADMAP.md`: AI0-AI8 architecture and safety;
- `qa/ASIO_INPUT_ACCEPTANCE.md`: mandatory Windows ASIO acceptance;
- `qa/REKORDBOX_STREAM_DECK_PEDAL_ACCEPTANCE.md`: mandatory wired any-deck DJ Agent,
  Track mapping, absolute Timeline Loop, Release, and peer/hardware acceptance;
- `qa/M4_IO_VALIDATION.md`, `qa/M5_RELIABILITY_VALIDATION.md`, and
  `qa/M6_RELEASE_VALIDATION.md`: physical I/O, soak, and release evidence;
- `RELEASE_STATUS.md`: historical release evidence and external blockers;
- `qa/CODEX_HANDOFF_2026-08-19.md`: historical implementation checkpoint log.

Older percentages, resume instructions, and v1.0/v1.1 completion statements are
historical when they conflict with the post-alpha.10 handoff, `AGENTS.md`, or this
registry. In particular, the 2026-08-13 75.5% planning roll-up is not current
release evidence, and its old Media A1 resume point must not replace the bounded
residual selected through the current handoff.

## 2. Completion decision

Syndocal is complete only when all of the following are true for one frozen tag and
the exact artifacts built from it:

1. every Supported row has Implemented, independently Reviewed, and required
   Automated, Native, Hardware, Security, Migration, and Distribution evidence;
2. no P0-Code, P0-Release, P1, or release-blocking P2 remains;
3. all Deferred, External, and Out-of-scope rows are explicit and do not contradict
   an advertised claim;
4. first-party build/test/package warnings are zero for the supported matrix;
5. product version, tag, package metadata, updater manifest, filenames, checksums,
   release notes, and evidence manifest agree;
6. Q1 traceability, Q2 decisions, Q3 risks, Q4 evidence, and Q5 completion query in
   the master roadmap are current and point to that tag/artifact set.

Do not report one ambiguous overall percentage. Report at least software
implementation, automated proof, native/hardware proof, distribution/legal
readiness, the affected domain, and the next blocking gate. A P0 blocker overrides
all percentages.

## 3. Mandatory workflow for every tranche

Every implementation tranche uses this flow. A later tranche may not consume an
unreviewed or unverified predecessor.

1. Read `AGENTS.md`, the post-alpha.10 handoff, this registry, and the selected
   domain authority fully.
2. Run `git status --short`, `git log -3 --oneline`, compare local/remote HEAD, and
   preserve all unrelated or user-owned changes.
3. Write the requirement, non-goals, risk class, compatibility/migration boundary,
   file ownership, exact tests, native/hardware needs, and stop condition.
4. Keep decomposition, instructions, integration, and completion claims with Sol.
   Use the capability hierarchy Sol > Ox-alpha (`opencode/x-preview-f-free`) >
   Terra > Luna, make Ox the default delegate for bounded implementation,
   investigation, and review, and assign explicit files. Use Luna Max only for
   small, explicit, low-ambiguity units already decomposed by Sol. A Terra
   implementation requires an independent Ox adversarial review before integration.
5. Assign a separate adversarial reviewer, defaulting to an independent Ox session.
   The reviewer is read-only until the implementation owner reports a stable
   checkpoint; implementer self-review is never sufficient.
6. Parallel execution and elimination of avoidable elapsed time are mandatory.
   Continuously fill every safely independent available lane with a
   capability-appropriate assignment. While agents/builds/external I/O run, advance
   non-overlapping implementation, investigation, test planning, documentation, or
   evidence work. Leave a lane idle only when no safe productive task exists and
   record why at the next checkpoint. Serialize only true dependencies, exclusive
   UI/native operations, destructive actions, or same-file ownership, and never let
   agents edit the same files concurrently.
7. Implement one bounded unit. Do not mix refactors, warning cleanup, schema change,
   UI change, and unrelated features in one commit unless they form one indivisible
   correctness boundary.
8. Run focused gates with nonzero selected tests, then neighboring regressions,
   format/static/localization/viewport checks, and warning comparison.
9. Freeze the diff and obtain independent P0/P1 review. Fix and re-review; a summary
   from the implementer is not review evidence.
10. For native UI/runtime changes, immediately before the release build resolve the
    exact checkout executable path, stop only that exact process, run
    `pnpm --dir app tauri build --no-bundle`, launch the exact executable, verify
    exactly one responsive `Syndocal` window, and maximize it before UI actions.
11. Record exact commands, counts, artifact/process/window evidence, warnings,
    unverified hardware/external rows, and non-claims in roadmap/status/handoff.
12. Run `git diff --check`; stage an explicit file list; inspect
    `git diff --cached --name-only` and `git diff --cached --check`.
13. Commit with a meaningful Conventional-style subject, push the active branch,
    and verify local HEAD equals the upstream remote. If push fails, preserve the
    commit and record the failure and recovery action.
14. When a handoff must name the implementation hash, use a small documentation
    follow-up commit, push it, then leave the worktree clean.

No chat-only progress is a checkpoint. No frontend-only or unit-only run is native
evidence. No unavailable physical device is a passing hardware result.

## 4. Product version and schema version policy

### 4.1 Current release train

The active train advanced from the long-lived `1.1.0` metadata through accepted
alpha checkpoints; current product metadata is `1.2.0-alpha.38` on
`codex/syndocal-v1.2`. Alpha.38 is the active source tranche and repairs the
Timeline CUE transport-authority/Guide-retirement boundary found during
real-machine authoring. The complete current source, native, show-output,
audio, DJ, and remaining physical acceptance state is recorded in
`qa/SYNDOCAL_SHOW_COMPLETION_HANDOFF_2026-08-28.md`.

Alpha.38 native artifact identity, launch/maximized-window proof, audible
media/Click/Guide confirmation, Unity/GPU, camera, DJ reconnect/pedal, and
physical output acceptance remain unverified until their explicit gates run.
Alpha.37 and earlier native artifacts are historical evidence only and must not
be relabeled as alpha.38 acceptance.

The immediately preceding alpha.25 source/native checkpoint remains immutable
historical evidence: source/native evidence was
`566a7101b0d5c9307e8d0efa5ccf499aba3eb404`; the exact MSVC 14.44 linker was
first, and `pnpm --dir app tauri build --no-bundle` passed in `3m06s` with
first-party warnings `0`. Vite transformed `297` modules, emitting
`App-CUUI4pgY.js` (`498.21 kB`) and `gdtfProfileActions-54y__lyr.js`
(`3.91 kB`). Its artifact was `target/release/syndocal.exe`,
Product/FileVersion `1.2.0-alpha.25`, `59,807,744` bytes, SHA-256
`D42B0A1B245197A76E3D3B5CC45DEEB1C6C33EDE6261C7DA94C34147141239C1`,
LastWrite `2026-08-28T06:20:27.6424691+09:00`, PID `83252`, handle
`60826240`, title `Syndocal`, and responsive/maximized state verified. Native
I/O DMX full-width selector/workbench and Setup Lighting/Profile rendering were
accepted. `qa/harnesses/run-syndocal-three-display-show-acceptance.ps1` remains
the StandardRelease authority for that exact historical alpha.25 version,
`59,807,744`-byte size, SHA-256, and source HEAD; PowerShell 7 and Windows
PowerShell 5.1 self-tests both passed `88/88`, with syntax passing in both. No
Apply or hardware acceptance was run. DJ TCP re-established the `.50.2` peer,
but `Untitled.sdc`/its mapping was not authoritative. The preceding alpha.23 source checkpoint remains
historical at exact committed/pushed, upstream-equal source
`5e7d27df7f5864449d4838782f6eca2f9b81d360`; alpha.22 authoritative Scene
source evidence remains preserved in historical section 55. The historical
alpha.21 Product/FileVersion was `1.2.0-alpha.21`, with its I/O/native source
retained in historical section 54.
The separate read-only show structural preflight is committed and pushed at
`b35d3ba351b29caaedfb7d0c6f4e29fc84580e83`.
`1.2.0-alpha.12` is the committed immutable prior checkpoint: its standard
MIT/WASAPI and local Show-ASIO artifacts were built, natively verified, and
hash-pinned from source checkpoint `ff61a6dec6eb5e4bc0993d9b65cd137fe1872aae`;
later display-stable-identity-QA, Show-ASIO hardlink-fix, cleanup
inaccessible-process-fix, and cleanup exact-gate checkpoints (`b543067`,
`fb25ab1`, `ef7b647`, and `c40cfd8`) intentionally did not advance product
metadata; and those alpha.12 artifacts are never rebuilt or replaced under their
version. `1.2.0-alpha.13` is the committed and pushed Timeline context-menu UI
checkpoint `bbb684cee4c8b01cfc019575569bd26835dbc732`; its exact native binary was
built and window-verified before this DJ-Link tranche. The historical alpha.14
advance synchronizes the strict DJ-Link v3 and Stage-1 no-response fallback
surfaces. Its committed and pushed runtime/native source checkpoint is
`92122f1b148d40845b2cfe3e4618a57ce132b3df`; the standard native artifact was
built by the required gate and window-verified from that source. It is not a
published tag or installer, and it does not close Show-ASIO or physical DJ
acceptance. The branch name may remain historical; artifact metadata and tags
must not derive a false version from it.

The historical `1.2.0-alpha.18` any-deck DJ-Link/source and synchronized
metadata checkpoint was committed at the exact `HEAD`/upstream
`db4eefc348b01ee05dd2dc87945afa85de8803e` on `codex/syndocal-v1.2`. Focused
MASTER clean-break gates passed protocol `7/7`, runtime `5/5`, single-thread
I/O `37/37`, frontend/build, live, and static checks; the locked full source
rerun passed `1164` / `0` / `11` for Syndocal pass/fail/intentional
hardware-media ignores with zero first-party warnings. Frontend invokes are
`422`, localization is `3559`, and the viewport/setup harness passes. Exact
MSVC live discovery passed and marks Ethernet4 `192.168.50.1` eligible. The
clean release gate `4fc443d` passed after staging seven pinned DLLs.

The alpha.18 native build and maximized-window launch gate are now verified with
the required MSVC 14.44 linker-first setup. Its artifact is
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`,
Product/FileVersion `1.2.0-alpha.18`, size `58,740,224` bytes, SHA-256
`841068E08F80EB877FBA919FB86D3F52B3D4444314B47ABEF995BFA593E8D4F9`,
LastWriteTimeUtc `2026-08-27T05:38:40.1840641Z`. Launch observed exactly one
responsive process, PID `80264`, title `Syndocal`, window handle `854080`,
`IsMaximized=true`, start `2026-08-27T14:38:57.8350806+09:00`. The last
committed alpha.17 source checkpoint is the pushed, upstream-equal
`fb5d18fdf898a1435bed173ddd17934a04a97897`. Its engine `dj_link_` proof is
`25/25`, Syndocal `dj_link_dispatch_` is `8/8`, the three-display harness is
`80/80`, the frontend build passed, detached `check:release` passed including
`169` ASIO packaging assertions, and first-party warnings are `0`. Its exact
native artifact is `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`,
Product/FileVersion `1.2.0-alpha.17`, size `59,021,824` bytes, SHA-256
`8B35A0F89ED6FA9A1BF8B1929BFA323F7F6250DF059D6314CCE7DDD6D39EBE45`, with
exactly one responsive maximized Syndocal window at PID `57640`. This alpha.17
artifact/process evidence is historical and is not re-bound to alpha.18. The
external rb-output v1.1.8 source is pushed and clean at exact commit
`0f3e8c6851857c8542c132a89a7d44289002b1f5`; its stable suite passed `415` total
/ `413` pass / `0` fail / `2` intentional skips, including the focused
non-Master Deck 2 router-to-real-MIDI seven-byte proof at `12/12`. The verified
alpha.16/alpha.15 artifacts remain immutable historical evidence and are not
relabeled.

The synchronized product-version surfaces are:

- workspace version in `Cargo.toml` and first-party entries in `Cargo.lock`;
- `app/package.json`;
- `app/src-tauri/tauri.conf.json`;
- `app/scripts/check-release-metadata.mjs`;
- versioned artifact names in `app/scripts/bundle-macos-runtime.sh` and
  `.github/workflows/cross-platform.yml`;
- current-version user/release documentation and updater metadata.

Run `pnpm --dir app run check:release` after every version change. Before any
artifact is called a release candidate, add a gate proving the version is greater
than the previous release tag and exactly matches the tag, updater manifest,
artifact filenames, and embedded metadata.

### 4.2 Increment rules

- Ordinary internal commits do not require a version bump.
- Every intentionally distributed development artifact increments the prerelease
  ordinal (`alpha.1`, `alpha.2`, ...); never overwrite an artifact under one version.
- Move to `beta.1` only at feature freeze with all planned implementation present.
- Move to `rc.1` only after first-party warnings are zero and no P0-Code/P1 remains;
  later RC fixes increment the RC ordinal.
- Remove the prerelease suffix only after all P0-Release hardware, legal, signing,
  clean-machine, update, and publication gates pass.
- PATCH is backward-compatible correction, MINOR adds compatible product behavior,
  and MAJOR permits an intentionally migrated incompatible product contract.
- Tags are immutable. A rebuilt artifact receives a new version; it never replaces
  bytes under an existing tag/version.

Product SemVer is independent from `.sdc`, template, registry, command, API, ABI,
checkpoint, and cache schema versions. A schema version changes only when its own
compatibility contract requires it, with migration/golden/future-version rejection
tests. A product bump never authorizes an untested schema bump, and a schema bump
never substitutes for advancing the product version.

`tools/asio-bridge/Cargo.toml` currently uses `1.0.0` as the bridge package/ABI
line, not the Syndocal product version. It is not permitted to label a distributed
Syndocal artifact. Before ASIO distribution, the release manifest and
`check:release` must map the exact product version to the separately versioned bridge
ABI/package, DLL hash, SDK/license decision, and compatible app loader. A bridge ABI
bump follows ABI compatibility rules; a product artifact still follows Product
SemVer and receives its own unique version.

## 5. Warning elimination policy

Warnings are work, not harmless release log noise. The following ratchet begins
immediately:

Historical snapshot — not the current state: the provisional observed baseline
came from the fresh `1.2.0-alpha.1` native build, where the Engine reported 9
warnings, the Syndocal release target reported 58 warnings, and Vite emitted its
oversized-chunk warning for multiple generated chunks. Those counts were
log-level starting evidence for that ordinal only, are not the complete W0
feature/platform inventory, and must never be used as an allowlist or cited as a
current result. Every zero-first-party-warning result is a configuration- and
commit-pinned historical measurement, never a standing property of this tree:
the enforced Windows rows (default/all-targets, tests, release, Spout,
frontend, native release) were measured at zero at the 2026-08-25 exact-linker
alpha.11 checkpoints (section 37 at D4-checkpoint HEAD
`63cf795d17846602419d63a007db9f3a95cfce7b` and section 39 at merged HEAD
`b4a5b62ad48c7e3e58f78c66cf2914e0f53a46f5`) and in individual focused gates at
their own recorded commits, each valid only for that frozen configuration and
tree. The committed `1.2.0-alpha.12` checkpoint was measured at source
checkpoint `ff61a6d` with zero first-party warnings across its recorded Windows
configurations (2026-08-26 pause handoff), superseding that tree's earlier
12 open first-party large-error lints (near-show finding FC-28). The historical
`1.2.0-alpha.14` standard native gate recorded zero baseline/current warnings
at source checkpoint `92122f1b`; the broader alpha.12 configuration matrix has
not been rerun in full on alpha.14. W0 must still recount every supported
feature and platform configuration — including the two pending macOS rows —
and classify every first-party diagnostic.

1. **W0 inventory:** capture warning code, file/line, target/feature, owner, reason,
   and removal checkpoint for default release, tests, all targets, ASIO, NDI, Spout,
   macOS, and Linux. Separate third-party/build-tool warnings from first-party code.
2. **W1 no growth:** no checkpoint may add a first-party warning. A warning in a
   modified file is a tranche blocker. Handoff records baseline/current/delta.
3. **W2 removal:** use focused commits to remove unreachable compatibility code,
   narrow `cfg` boundaries, delete obsolete helpers, or add real production use.
   Do not hide debt with crate-wide `allow(dead_code)`, rustc command-line
   warning-allow flags, fake reads, or
   arbitrary Vite limit increases. A narrow allow requires justification, owner,
   upstream link when applicable, and expiry.
4. **W3 zero gate:** before beta, all supported first-party configurations must pass
   warning-free. Enable warning-as-error in CI only after the baseline reaches zero,
   then keep it permanent.
5. **W4 frontend/package:** TypeScript, Vite, Tauri, installer, and updater logs must
   have no unowned warning. Split or lazy-load oversized chunks based on measured
   startup/operator performance; do not merely raise the threshold.

Required warning commands eventually include:

```text
cargo check --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
pnpm --dir app exec tsc --noEmit
pnpm --dir app build
pnpm --dir app tauri build --no-bundle
```

Repeat the Rust/native gates for each supported feature/platform combination. A
temporary third-party allowlist is a versioned release artifact with expiry; it may
not contain warnings originating in repository first-party source.

### 5.1 Bounded W1 checkpoint (historical commits; superseded by later zero-warning rows)

Implementation commit `3ee303f4ce7fed897e4d2473ddf80b4335b20591`
establishes the reviewed Windows warning ratchet for four configurations: default
all-targets 83/67 warnings/identities with 11/11 artifacts, release 83/67 with
11/11, tests `--no-run` 25/21 with 11/11, and isolated Spout 79/67 with 9/9.
The 52-group negative harness and independent review found no remaining P0/P1 in
that bounded gate. Native release verification at that checkpoint reproduced
Engine 9, Syndocal 58,
and one Vite chunk warning without growth.

The focused warning/P2 checkpoint
`a228ba5e492841618ce0262038c965b5519d1ddd` reduced the then-current first-party
occurrences without editing the baseline: default and release 83 -> 61, workspace
tests 25 -> 20, and isolated Spout 79 -> 57, with unchanged exact artifact coverage.
At that checkpoint the native release reported Engine 9 and Syndocal 41 warnings plus the same
Vite chunk warning. Both figures are historical: later focused commits and the
enforced Windows rows reached zero first-party warnings (see section 14 and the
2026-08-25 exact-linker checkpoint), so neither commit's counts describe the
current tree. Its independent review returned P0 0, P1 0, and P2 0 for the
bounded Take Over repair. The still-disconnected Output Lease module, including
`checked_deadline`, remains owned AI3 integration debt and is not suppressed or
deleted in isolation.

This does not check the Phase 0 W0 box. Nine configurations remain pending in the
versioned inventory: ASIO loader, excluded ASIO bridge, NDI, macOS default/release,
Linux default/release, frontend structured warnings, and native-release structured
warnings. `requiredMatrixComplete` therefore remains false. H1 uses a reviewed
one-time local bootstrap. Docs-only H2
`33f58df2f69c45892f2089037cbf00ed0aab3e56` has H1 as its parent and changes only
the two checkpoint documents. Normal-mode verification with `base=H1` and
`head=H2` passed all four configurations at 83/83, 83/83, 25/25, and 79/79 warning
occurrences, with exact artifact coverage 11/11, 11/11, 11/11, and 9/9. GitHub
Actions run `32351887066` executed zero steps because account billing/spending limits
blocked every job, so no CI/platform claim is derived from that run.

## 6. Dependency-ordered remaining implementation

`qa/SYNDOCAL_COMPLETION_LEDGER.json` is the machine-readable open-item index for
sections 6 through 9. It records exactly 50 `Open` current checkboxes and 8
`Deferred` rows: the two explicit platform deferrals plus the six frozen
distribution rows. Each carries one stable trailing `completion-ledger` marker
whose `Open` or `Deferred` kind is checked against the ledger. The index is
additive: it records dependencies, evidence paths, and non-claims without
changing a row's open/accepted truth or replacing the master roadmap's Q1-Q5.

### Phase 0 — Rebaseline, version, warnings, and ledgers

- [x] Advance and verify all product metadata as `1.2.0-alpha.1`.
- [x] Replace stale resume instructions and percentages with current AI3 truth.
- [ ] Build Q1-Q4 coverage from every phase below; assign Supported/External/etc. <!-- completion-ledger: Open: COMP-Q1-Q4-001 -->
- [x] Create the structured W0 inventory and enforce the W1 no-new-warning ratchet.
- **Deferred outside the Windows target:** macOS dev/release warning enforcement. <!-- completion-ledger: Deferred: WARN-MACOS-001 -->
  The global inventory may remain 11 enforced / 2 pending without blocking the
  current Windows completion target.
- [ ] Extend `check:release` with tag/previous-version/updater/artifact checks. <!-- completion-ledger: Open: RELEASE-METADATA-GATE-001 -->

Exit: synchronized version metadata, warning baseline, current traceability/risk/
evidence ledgers, clean reviewed commit, and pushed handoff.

### Phase 1 — Generic project authority and atomic project mutations

- [x] E1 generic Begin reply-loss, terminal recovery, live-owner transaction
  liveness, renderer retirement, and stale delayed Commit/Cancel rejection.
- [x] E2 authority bundle/generation consistency across every project mutation.
- [x] E3 recovery durable handshake and crash/reply-loss behavior.
- [x] E4 Save/Save As/template/backup reservation, journal, mapping flush, atomic
  replacement, and truthful terminal receipt.
- [x] D1 machine/session cache read purity.
- [x] D2 atomic PATCH/GDTF Repair with whole-batch prevalidation, Published ACK,
  allocator/cache/output rollback, and one history result.
- [x] D3 server-side admission for every mutation, including raw Tauri paths.
- [x] D4 Stage import/mutation identity fence, atomicity, Undo, and error truth.
  Accepted at the 2026-08-25 alpha.11 native checkpoint (section 37);
  fixture/hardware-specific Stage behavior remains an open physical boundary.

Exit: no project/file mutation bypasses one recoverable authority/publication path.

### Phase 2 — Complete AI3 runtime/output bridge

- [x] Extend the pure lease core into a bounded multi-lease registry with atomic
  Lighting/Video overlap handling.
- [x] Add process-local exact request receipts, same-ID/different-shape rejection,
  bounded lanes, rate limit, audit truth, and restart non-reclamation.
- [ ] Add crash-safe/durable terminal recovery rather than reclaiming pre-crash <!-- completion-ledger: Open: AI3-DURABLE-RECOVERY-001 -->
  authority or presenting process-local receipts as durable truth.
- [x] Wire lease state into AppState, generation-stamped query, canonical commands,
  registry metadata, and current-process owner retirement.
- [x] Revalidate exact lease owner/resource/generation/expiry at the final existing
  R4 Release/Arm/Take Over commit boundary while S0 remains independent.
- [x] Orphan affected leases on project identity replacement without physical change.
- [x] Canonicalize or fail-close code-side MIDI, OSC, DMX mapping, Remote, shortcut,
  all/video/per-output release, and every other discovered energizing legacy route.
- [ ] Verify those ingress policies through native clients and hardware rather than <!-- completion-ledger: Open: AI3-NATIVE-INGRESS-001 -->
  treating the generated source inventory as physical acceptance.
- [x] Fence project replacement through candidate/orphan receipts, generation checks,
  and an explicit re-Arm requirement.
- [ ] Prove New/Load/Recovery/Backup/Take Over retirement and re-Arm end to end with <!-- completion-ledger: Open: AI3-PHYSICAL-REARM-001 -->
  acknowledged physical output state.
- [x] Prove code-side stale owner/ABA, restart non-reclamation, transfer races,
  bounded 10,000-call saturation, and candidate/teardown ACK behavior.
- [ ] Prove durable reply-loss/crash recovery, native dangerous-action <!-- completion-ledger: Open: AI3-DURABLE-ACCEPTANCE-001 -->
  Yes/No/close behavior, physical creation/teardown ACK, and the five-display
  hardware behavior. The retired Raw Input challenge is not an acceptance item.

Exit: all five AI3 roadmap categories accepted. Do not begin AI4 before this exit.

### Phase 3 — AI0-AI8 control plane completion

- [ ] AI0 complete source inventory/coverage gate; unclassified mutations fail. <!-- completion-ledger: Open: AI0-COVERAGE-001 -->
- [ ] AI1 query/event schemas, snapshots, generations, gap/resnapshot, bounds. <!-- completion-ledger: Open: AI1-SCHEMAS-001 -->
- [ ] AI2 authored command bridge with E/R/H, owner incarnation, receipts, Undo. <!-- completion-ledger: Open: AI2-COMMAND-BRIDGE-001 -->
- [ ] AI4 principals, pairing, grants, revocation, kill switch, exact reviewed <!-- completion-ledger: Open: AI4-CONSENT-001 -->
  local/native consent policies, and release-build bypass absence. This does not
  restore the retired Raw Input/six-digit/Enter challenge.
- [ ] AI5 authenticated localhost sidecar, MCP, JSON-RPC/REST, WebSocket, discovery, <!-- completion-ledger: Open: AI5-SIDECAR-001 -->
  bounded queues, process lifecycle, and no cached authority replay.
- [ ] AI6 administration UI for principals/grants/revocation/audit/health. <!-- completion-ledger: Open: AI6-ADMIN-UI-001 -->
- [ ] AI7 adversarial parity/security/rate/reply-loss/fuzz/saturation proof across <!-- completion-ledger: Open: AI7-ADVERSARIAL-PROOF-001 -->
  Tauri, MIDI/OSC/DMX, Remote, shortcuts, API, and MCP.
- [ ] AI8 release-native real external clients, clean install, hardware output, <!-- completion-ledger: Open: AI8-EXTERNAL-ACCEPTANCE-001 -->
  crash/restart, security review, and artifact inspection.

Exit: AI0-AI8 accepted without weakening local Blackout or claiming unattended R4/R5.

### Phase 4 — Output ownership, project swap, and ShowClock decision freeze

- [ ] F1 monotonic input generations for MIDI/OSC/DMX Learn/workers and stale callback <!-- completion-ledger: Open: F1-INPUT-GENERATIONS-001 -->
  retirement on mapping/project replacement.
- [ ] F2 full Lighting/Video/Both/Standby ownership across DMX, NDI, Spout/Syphon, <!-- completion-ledger: Open: F2-OUTPUT-OWNERSHIP-001 -->
  Display, native windows, SDK resources, Take Over, teardown ACK, and explicit Arm.
- [ ] Freeze ShowClock transport/discovery/authentication/key rotation/replay, <!-- completion-ledger: Open: SHOWCLOCK-DECISIONS-001 -->
  master-clock/slew/Hold, witness/fence/physical-interlock, mixed-version, and
  supported-network decisions.

Exit: local/project output ownership and replacement are accepted, and ShowClock
decisions are frozen. Distributed implementation remains blocked until Phase 5
Audio/recording/live-source clock and ownership semantics are complete.

### Phase 5 — Video, Timeline, media-derived data, audio, recording, live sources

- [ ] Reintegrate accepted Windows Clip Slot/Layer Bus/FX tranches into the full gate. <!-- completion-ledger: Open: VIDEO-FULL-GATE-001 -->
- [ ] C2 Clip Take and C4 mapping/Timeline integration. <!-- completion-ledger: Open: VIDEO-C2-C4-001 -->
- [ ] L-TL5 Follow/crossfade, BPM slew, failure policy, and `Trans`/`Complete` Guide. <!-- completion-ledger: Open: TIMELINE-FOLLOW-001 -->
- [ ] L-TL7 Undo/Redo/save/reload group selection/focus, fixed Guide device <!-- completion-ledger: Open: TIMELINE-PERSISTENCE-001 -->
  routing, native A/V/Lighting synchronization, fault, and viewport proof.
- [ ] M thumbnail/waveform/proxy/analysis identity, bounded background workers, <!-- completion-ledger: Open: MEDIA-DERIVED-001 -->
  cancellation, cache/eviction, predecode/prefetch/degraded operation, and cold/warm
  cache performance budgets.
- [ ] L authored Audio schema/migration/history and explicit ShowClock/audio/PTS <!-- completion-ledger: Open: AUDIO-AUTHORED-001 -->
  master-clock, resampling/slew/seek/loop/underrun/device-fault policy.
- [x] Extend the existing Web Remote listener with authenticated DJ Link events. Map
  `DJ_TRACK_ACTIVE` to project-owned Timeline starts, converge authored A-B
  Loop from absolute `DJ_LOOP_STATE`, and make `DJ_RELEASE` idempotently turn
  the DJ loop OFF and relinquish DJ clock authority without changing transport,
  position, playing state, child transport, or Follow; the local clock continues
  from the current playhead.
  Do not add a second Agent/server or send rekordbox MIDI from Syndocal.
- [ ] Stable live camera/screen/NDI/Spout/Syphon/generator identity and availability, <!-- completion-ledger: Open: VIDEO-LIVE-SOURCES-001 -->
  permission/fault/reconnect truth, and old-worker retirement.
- [ ] Recording `Idle -> Preparing -> Recording -> Finalizing -> Complete|Fault`, <!-- completion-ledger: Open: RECORDING-001 -->
  target reservation, disk/crash/encoder/timeout recovery, verified atomic artifact,
  two-PC ownership, and authoritative asset import.

Exit: maximum-condition one-hour A/V/Lighting/recording proof meets fixed budgets.

### Phase 6 — ShowClock, UI, Remote, security, migration, and supportability

- [ ] Implement ShowClock schema/simulator, authenticated peer sync and estimator, <!-- completion-ledger: Open: SHOWCLOCK-IMPLEMENTATION-001 -->
  timestamped exactly-once actions, project/lease/audio/recording generation coupling,
  witness/fence, UI, and two-process then two-machine fault/soak proof.
- [ ] H1 reachability: every supported feature has a discoverable operator path and <!-- completion-ledger: Open: UI-H1-REACHABILITY-001 -->
  no dead/hidden route that only an internal command can reach.
- [ ] H2 shared shell: one truthful navigation/status/error/selection/focus contract <!-- completion-ledger: Open: UI-H2-SHELL-001 -->
  across Setup, Edit, Control, Touch, native windows, and compact/full layouts.
- [ ] H3 Setup completion: Patch/GDTF/OFL, mapping, I/O, output/device configuration, <!-- completion-ledger: Open: UI-H3-SETUP-001 -->
  validation, empty/error states, keyboard/pointer reachability, and native proof.
- [ ] H3 DJ Link setup: explicit Show-LAN NIC/bind address, dedicated token rotation, <!-- completion-ledger: Open: DJ-LINK-SETUP-001 -->
  connection/session diagnostics, current admitted owner deck, project Track-to-Timeline
  mapping CAS, disconnect recovery, and `Use Current Track`. No Pedal/MIDI controls.
- [ ] H4 Edit completion: Media, Timeline, Phase/Guide/loop/group/follow, FX, Stage, <!-- completion-ledger: Open: UI-H4-EDIT-001 -->
  history/Undo/Redo, import/relink, save/reload focus, and native proof.
- [ ] H5 Control completion: live Lighting/Video/Audio, Cue/Clip/Take/Transition, <!-- completion-ledger: Open: UI-H5-CONTROL-001 -->
  Blackout/Arm/Take Over, recording, diagnostics, failure/recovery, and native proof.

- [ ] N Remote/Touch LAN/TLS exposure, pairing, Origin/Host, grants, rate/size bounds, <!-- completion-ledger: Open: REMOTE-SECURITY-001 -->
  RDM/TOD cancellation/ownership, parser/path/archive fuzz, dependency/SBOM/redaction.
- [ ] O supported `.sdc`/template/cache/protocol version matrix, golden migration <!-- completion-ledger: Open: MIGRATION-COMPATIBILITY-001 -->
  corpus, hostile/corrupt input, fuzz, backup/recovery/upgrade compatibility.
- [ ] P generation-stamped status, redacted diagnostic bundle, updater wrong-channel/ <!-- completion-ledger: Open: OBSERVABILITY-SUPPORT-001 -->
  downgrade/signature/corruption/offline/rollback behavior, and operator/support
  startup/failure/takeover/recovery/shutdown runbooks.
- [ ] H native NVDA, High Contrast, color-independent states, 125/150/200% scaling, <!-- completion-ledger: Open: ACCESSIBILITY-NATIVE-001 -->
  keyboard-only safety workflows, IME, dialog/popout focus, and reduced motion.

Exit: supported exposure, migration, recovery, accessibility, and diagnostics have
native evidence, not only browser/static proof. ShowClock partition/crash/rejoin
cannot create simultaneous output; automatic failover is not claimed without the
accepted strongly consistent witness/fence or physical interlock.

## 7. Mandatory Windows ASIO completion

ASIO is a Windows product-release requirement while remaining a separately licensed,
non-default artifact from MIT/WASAPI.

- [ ] Select GPLv3-separated artifact or Steinberg proprietary SDK agreement; package <!-- completion-ledger: Open: ASIO-LICENSE-001 -->
  notices/source obligations and approve installer/updater separation.
- [x] Pass a second vendor's working driver (HOTONE 44.1 kHz / 2-channel / i32 /
  128-frame, 100 exact Start/Stop/Free cycles with no fallback).
- [ ] Pass advertised 44.1/48/96 kHz, 64/128/256 frames, format, mono/stereo/channel <!-- completion-ledger: Open: ASIO-FORMAT-MATRIX-001 -->
  selection matrix without silent fallback.
- [ ] Pass occupied-driver, control-panel rate/buffer change, reset/resync, XRUN, <!-- completion-ledger: Open: ASIO-FAULT-MATRIX-001 -->
  unplug/replug, callback-gap, and no-callback fail-closed/recovery matrix.
- [ ] Pass matched one-hour ASIO and WASAPI soak with overrun 0, callback p99 below <!-- completion-ledger: Open: ASIO-SOAK-001 -->
  20% and max below 50% of buffer duration, capture-to-engine p95 <= 40 ms, and
  loss-to-zero <= 250 ms.
- [ ] Measure physical input-to-pixel latency and five matched TouchDesigner trials. <!-- completion-ledger: Open: ASIO-LATENCY-001 -->
- [ ] Verify selection persistence, stale/ambiguous device lock, native UI telemetry, <!-- completion-ledger: Open: ASIO-PERSISTENCE-PACKAGE-001 -->
  artifact feature identity, and absence of fallback in the final package.

Exit: every checkbox in `qa/ASIO_INPUT_ACCEPTANCE.md` is checked with exact artifact,
driver/device, raw logs, operator, date, and measurement source.

## 8. External hardware, comparative, platform, and venue acceptance

- [ ] Real Art-Net/sACN nodes and fixtures: addressing, RGB/wheel, Pan/Tilt, intensity, <!-- completion-ledger: Open: DMX-ARTNET-001 -->
  strobe, Blackout, reconnect, multi-universe, switch topology, and 44 Hz continuity.
- [ ] Enttec USB PRO/DMXKing long run; Open DMX logic-analyzer Break 176 us, MAB 16 us, <!-- completion-ledger: Open: DMX-USB-RDM-001 -->
  frame period/failure; RDM/TOD discovery/correlation/timeout/cancel/removal.
- [ ] Physical MIDI Note/CC/Clock/MTC/feedback/All Notes Off; OSC and TouchOSC/iPad/ <!-- completion-ledger: Open: INPUT-PHYSICAL-001 -->
  Android Remote over wired/Wi-Fi with p50/p95/p99/max latency.
- [ ] Physical DJ-PC Pedal -> `rekordbox-DJ-Link-ForPCDJ` local MIDI plus wired <!-- completion-ledger: Open: DJ-LINK-HARDWARE-001 -->
  Agent -> Syndocal acceptance: preload/non-playing non-trigger, exact any-deck Track start,
  absolute repeated Loop divisions, Filter isolation, idempotent Release, local
  operation during disconnect, State Sync, ACK, and next-use readiness, with exact
  device/software/repository/NIC/mapping/timestamp evidence.
- [ ] Dual display/HDMI/fullscreen/DPI/refresh/unplug/reorder/GPU reset; NDI/Spout/ <!-- completion-ledger: Open: VIDEO-PHYSICAL-001 -->
  Syphon, camera, and screen-capture fault and one-hour matrices.
- **Deferred outside the Windows target:** macOS/Linux real-machine display, <!-- completion-ledger: Deferred: PLATFORM-MAC-LINUX-001 -->
  media, audio, project save/reload, and package acceptance.
- [ ] Venue GPU maximum ISF/layer/Preview/Program/output/recording one-hour run with <!-- completion-ledger: Open: VENUE-SOAK-001 -->
  frame/drop/CPU/GPU/RAM/VRAM/operator-response logs.
- [ ] Two-machine real-switch crash/restart/partition/rejoin/device-loss rehearsal <!-- completion-ledger: Open: SHOWCLOCK-VENUE-001 -->
  with zero simultaneous output and operator runbook.
- [ ] Pinned Daslight/SynapseRack/TouchDesigner version/license/hardware/content task <!-- completion-ledger: Open: COMPARE-PINNED-001 -->
  comparisons, preserving first failures, unmeasured rows, timings, operations, and
  synchronized output evidence. Never infer parity from counts or loopback.

## 9. Distribution, legal, clean-machine, and publication — deferred

The six former completion checkboxes in this section are outside the active
Windows-local target: platform/package matrix, Authenticode/Apple signing,
BOM/SBOM and notices, clean-machine install/upgrade/uninstall, signed updater,
and public tag/artifact/evidence publication. Reopen them only when distribution
becomes a product goal; they are not counted in the active 71-item denominator.

The following retain those six frozen rows as individually addressable
`Deferred`/out-of-scope items. They do not reopen distribution work; their
markers are deliberately checked by the completion-ledger gate.

- **Frozen / out of scope for the current Windows-local completion target:** Platform/package matrix. <!-- completion-ledger: Deferred: DIST-PLATFORM-PACKAGE-001 -->
- **Frozen / out of scope for the current Windows-local completion target:** Authenticode/Apple signing. <!-- completion-ledger: Deferred: DIST-SIGNING-001 -->
- **Frozen / out of scope for the current Windows-local completion target:** BOM/SBOM and notices. <!-- completion-ledger: Deferred: DIST-NOTICES-001 -->
- **Frozen / out of scope for the current Windows-local completion target:** Clean-machine install/upgrade/uninstall. <!-- completion-ledger: Deferred: DIST-CLEAN-MACHINE-001 -->
- **Frozen / out of scope for the current Windows-local completion target:** Signed updater. <!-- completion-ledger: Deferred: DIST-UPDATER-001 -->
- **Frozen / out of scope for the current Windows-local completion target:** Public tag/artifact/evidence publication. <!-- completion-ledger: Deferred: DIST-PUBLICATION-001 -->

## 10. Warning cleanup workstream

Run in parallel only where file ownership does not overlap the active feature tranche.
Suggested focused sequence:

1. remove obsolete first-party unused imports/functions and historical compatibility
   helpers that have no supported caller;
2. correct feature/platform `cfg` so optional NDI/Spout/Timeline code is compiled only
   where used, with feature-matrix tests preventing accidental removal;
3. wire or deliberately defer the new output-lease core before beta so it is not
   permanent dead code;
4. resolve Engine dead fields/methods through real use or deletion, not fake reads;
5. split/lazy-load Vite chunks and remeasure startup/operator latency;
6. reach zero in default check/test/release, then ASIO/NDI/Spout and OS matrices;
7. enable permanent CI warning-as-error and reject any allowlist growth.

Each warning commit records before/after counts, exact configurations, removals,
behavioral non-change proof, and the next owner. Warning cleanup never substitutes
for domain acceptance.

## 11. Final integrated verification and stop condition

After every phase above is committed and its per-tranche native/hardware gate is
closed, execute master roadmap section 22.2 without omission against one release tag:

1. Rust format, focused tests, workspace tests/check/clippy warning-free;
2. TypeScript, every static/localization/viewport gate, diff/link checks;
3. frozen adversarial review with no P0/P1/release-blocking P2;
4. exact-process native release build, one responsive maximized window, full native
   UI/accessibility workflows;
5. physical K/ASIO matrix, cold/warm performance and extended soak;
6. distributed ShowClock/two-PC fault matrix;
7. Windows migration/corruption/security matrices;
8. pinned commercial benchmarks against the verified Windows executable;
9. Q5 query against the exact Windows checkpoint, logs, and evidence manifest.

Distribution signing/updater/clean-machine/publication and macOS/Linux rows are
explicitly deferred by the Windows-only scope above and are not final-stop gates.

Stop and report rather than claiming completion if any command selects zero tests,
any warning is unowned, any artifact differs from the evidence hash, any external
gate is unavailable, or any claim exceeds the proven platform/hardware boundary.

## 12. Historical exact resume point — do not execute

This was the resume order at the 2026-08-21 Windows-first checkpoint. It is
historical and must not be executed as a current plan. Follow `AGENTS.md` and the
current phase status in this flow; consult the post-alpha.10 snapshot for exact
checkpoint evidence and frozen companion ownership only.

1. preserve the zero-warning Windows/Linux rows and W1 ratchet; measure the two
   macOS rows before claiming W0, beta, or RC warning acceptance;
2. do not restore the retired Raw Input/six-digit/Enter/15-second feature;
3. verify one-click local exact-`Both` Enable, create all five detected displays
   with exact persisted identities, then leave editor + LED panel + projector;
4. finish AI3 durable crash/reply-loss recovery and acknowledged project-lifecycle/
   physical-output tests before starting AI4;
5. keep distributed ShowClock, complete ASIO, signing/updater, beta/RC, and public
   release claims closed until their remaining gates below are proven.

## 13. 2026-08-21 bounded completion-flow update

The current AI3/DJ integration tranche has a frozen independent result of P0 0,
P1 0, and P2 0. The DJ transport reuses Web Remote, generation-fences the
irreversible handler boundary, recovers connection/admission state on handler
panic, distinguishes StateSync from triggers, and keeps all rekordbox/Pedal/MIDI
responsibility in `rekordbox-DJ-Link-ForPCDJ`. The topbar no longer carries the
Lighting/Video master sliders; no adjacent UI was reduced in size.

Historical checkpoint evidence only: the focused Rust and frontend gates listed in
the 2026-08-21 handoff passed, and that checkpoint's Windows native `--no-bundle`
build completed with zero first-party and zero Vite warnings. At that pinned
checkpoint, seven locally measurable rows were promoted and the inventory recorded
11 enforced / 2 pending. This is not a current-tree warning claim: macOS
dev/release remained pending external evidence, `requiredMatrixComplete` remained
false, and the dirty alpha.12 worktree must be measured again before any current
warning conclusion, beta, RC, or release acceptance.

The exact rebuilt executable was launched as one responsive maximized Syndocal and
five Display targets were enumerated. The Raw Input failure recorded at that time
is historical because the feature was later removed. Do not repeat a five-display
exercise for the active target; the final pass retains exactly the editor + LED
panel + projector state.

## 14. 2026-08-21 warning-P2 promotion result (historical checkpoint only)

At the 2026-08-21 promotion checkpoint, seven locally executable warning rows were
recorded as enforced with zero first-party warnings: Windows ASIO loader, NDI,
separately licensed ASIO bridge, frontend, native release, and Linux dev/release.
That commit/configuration-pinned promotion was reproducible through the
inventory-only `--promote-zero-warning` gate and host-specific execution; ordinary
baseline immutability was not relaxed. Its exact coverage is recorded in
`qa/warnings/warning-inventory.json`, with that checkpoint's required matrix at 11
enforced / 2 pending. Both remaining rows were macOS-only and therefore kept
`requiredMatrixComplete=false`; this was a bounded historical P2 resolution, not
beta/RC or whole-product completion. It does not establish a standing zero-warning
state for the dirty alpha.12 worktree, whose full checkpoint is unmeasured and has
12 open first-party large-error clippy lints (FC-28) pending a fresh full-gate
rerun.

## 15. 2026-08-21 Windows-first CI and VJ execution state

Windows is the active completion priority. The reviewed workflow now executes all
nine Windows-enforced warning rows and fails closed on missing FFmpeg, libclang,
NDI, or licensed ASIO SDK inputs. Exact-checkout process targeting is applied before
the native warning ratchet and the native bundle build. Independent review of the
frozen workflow returned P0 0, P1 0, and P2 0. This closes the remaining Windows CI
warning P2 item, but the two unmeasured macOS rows keep W0 cross-platform completion
and beta/RC acceptance open.

The implementation/documentation commit is
`85d6eb2b6d7877cd49801c288357d3c72a36046f` and is pushed to
`origin/codex/syndocal-v1.2`. Warning-ratchet self-test (61 groups), release
metadata, workflow YAML parsing, and `git diff --check` passed before commit.

The exact Windows native executable was responsive and the sole main Syndocal window
was maximized. Five enabled Display outputs were historically configured against
monitor indices 0..4 before the physical-input design was removed. That old setup
is not the active acceptance target. The remaining native pass opens exactly the
LED-panel and projector outputs beside the editor and verifies their screen origins,
fullscreen sizing, playback, close/reopen truth, and output authority.

## 16. Historical OutputControl Raw Input checkpoint (superseded)

Windows-first OutputControl integration now closes the code-side Ready handoff
race that previously converted a physical six-digit input near the original
deadline into `challenge was not found`. The backend publishes one monotonic,
bounded Ready deadline (`max(original, completion + 5 seconds)`); the frontend
keeps Pending expiry exact, accepts only that bounded Ready extension, and polls
through the one-shot consume window. Expiry, replacement, replay, device removal,
and synthetic-input rejection remain fail closed.

The P2 route proof now scans every production `#[tauri::command]` body for the
reviewed DMX, Display, NDI, Spout, and ownership-transition sink markers. A route
must be in the exact legacy-rejected inventory or the exact canonical R4/S0
inventory; an unknown command, an inventory removal, or a rejection moved after a
side effect fails the gate. This supplements, rather than replaces, the exhaustive
EngineCommand classifier and external MIDI/OSC/DMX/Web Remote adapter checks.
The frozen independent review returned P0 0, P1 0, and P2 0.

Focused evidence is 12/12 Raw Input tests with the interactive SendInput negative
test still separately ignored, 6/6 legacy-route tests, 19/19 OutputControl tests,
the eight-operation frontend controller contract, DJ Link and safety-blackout
contracts, 61 warning-ratchet self-test groups, release metadata, formatting, and
`git diff --check`. The final `pnpm --dir app tauri build --no-bundle` completed in
1m31s with zero first-party Rust warnings and zero Vite warnings. The exact release
executable has SHA-256
`D5CC6BF3908AE4295A134B21DDB7CF953D717BFF1AE52E68230EE27C7AC235A6`, is the
only responsive exact-path Syndocal process, and its sole main window is maximized.

Physical hardware acceptance is still deliberately open. A newly issued challenge
expired without an observed Ready/lease result; no synthetic input or automation
was substituted. The next action remains a coordinated human-keyboard lease
confirmation, a second confirmation for `Both` Arm, five fullscreen origin checks,
and the final editor + LED panel + projector state. Do not mark those boxes complete
until that native evidence is recorded.

## 17. 2026-08-21 Windows usability and physical-input continuation

Windows completion now also requires a simple normal path rather than exposing
implementation fields as the default operator workflow. Display-output creation
must enumerate the current native displays and make the normal path select one
display and add it. The display label, physical dimensions, and fullscreen default
are derived from that exact detected target. Manual dimensions, fade, endpoint, and
NDI/Spout/Syphon choices remain available in an initially closed advanced section.
The normal action must use a canonical OutputControl R4 operation with the exact
video lease, fresh physical consent, final fence revalidation, and an acknowledged
engine commit; the legacy `add_video_output` rejection is not relaxed. Empty,
failed, stale, or reordered display enumeration disables creation and cannot fall
back silently to monitor zero.

DMX, MIDI, OSC, and Web Remote likewise present connection state plus the actual
enable/start/stop/connect action first. Addresses, ports, protocols, mapping,
security limits, telemetry, and other engineering detail remain reachable through
closed disclosures. Active/Standby, output lease state, and the physical challenge
remain explicit safety controls rather than density debt. Existing typography,
controls, icons, spacing, and hit targets are not reduced; 1280x720 containment is
solved with reflow and internal scrolling.

The Scene Matrix Bank model must match the direct operator workflow: every Cue List
Bank, including an empty Bank, is rendered simultaneously as a horizontal column.
The one-row top strip only jumps/focuses an already visible Bank and contains Bank
creation plus the existing view controls; it is not a tab-panel switch. `+ Bank`
opens a focused input prefilled with the first unused `Bank N` and selects the whole
value, so Enter accepts the default and typing replaces it. Each Bank column owns
its `+ Scene`; it creates `New Scene` in that exact Bank even with zero patched
fixtures. Bank rename/delete are in the Bank context menu (mouse and Shift+F10),
and Bank drag reorder is one durable project transaction with one Undo. Lighting
Banks have no privileged `Main` row: ID 1 is an ordinary Bank, every Bank may be
renamed, reordered, or deleted, and only the last remaining Bank is protected.
Deleting a Bank deletes every Scene it contains; Scenes are not migrated to another
Bank. Selection, active/group/live/fade/timeline references to deleted Scenes must
be removed atomically and one Undo must restore the exact Bank and Scene state.
The separately defined video-composition `Main` is not part of this Bank model.
Cue group identity, live state, FX/Super Scene, cross-Bank cue drag, selection, and
project persistence otherwise remain intact.

The real keyboard retry in the maximized native app did not reach a challenge
receipt because the backend reported `Physical Raw Input registration query
failed`. An independent Win32 probe established the normal count/fill shapes as
`0/1/0` then `1/1/1/0`, which the current classifier already accepts. The classifier
therefore remains unchanged. The current diagnostic-only change returns a bounded,
secret-free numeric phase record for count/fill/stabilization failures; its focused
result was 16 passed, 0 failed, 1 separately ignored interactive SendInput test,
with zero first-party warnings. This is historical evidence only: OutputControl v2
subsequently removed Raw Input, six-digit, Enter, and 15-second challenges. Current
acceptance requires the one-click local Enable path, five stable-identity fullscreen
targets, and the final editor + LED panel + projector state.

The Bank implementation subsequently closed its frozen review at P0 0 / P1 0 /
P2 0. The final model has no privileged lighting `Main`: ID 1 is ordinary, only
the last Bank is protected, and deleting a Bank deletes all child Scenes plus every
runtime reference, including pending direct-child count-in. Save/reload and rollback
proofs are exact. Engine 767/769 (two ignored manual benchmarks), protocol 139/139,
strict control plane 73/74 (one ignored interactive SendInput), all three focused
five-viewport surfaces, frontend invoke 407, localization 3542/3542, and warning
zero passed. The native no-bundle build completed in 2m20s; executable SHA-256 is
`620EA06C9696EBD6B45E53D42840CD5806EC16D627D2ED3B89BFB0A05A6B1017`.
The sole exact-path process was responsive, but Computer Use could not bind its
window (`foreground window did not report a process id`) on two attempts. This is
an automation blocker, not evidence of maximize or display acceptance. Raw Input
was later removed and must not be resumed; only current OutputControl v2 native
window and display-placement acceptance remains open.
The reviewed implementation is commit
`262b8c0f43035ee44ff23cdfc195f4ac1a08b374`, pushed to
`origin/codex/syndocal-v1.2`.

## 18. 2026-08-21 Scene Matrix Bank visual-density acceptance

The Windows Scene Matrix normal surface now uses one fixed 156px border-box width
for every simultaneously visible Bank column and one fixed 72px border-box width
for every Bank jump control. Long names truncate visually while preserving their
full accessible label. The empty-column body contains no repeated Bank name and no
`No scenes in this bank` copy; the Bank header and exact-target `+ Scene` actions
remain. Scene cards inherit their visible identity treatment from the containing
Bank, including after cross-Bank movement, Undo, and project reload.

Acceptance evidence covers all five required Windows viewports for Scene Matrix
containment and cue drag, localization 3541/3541, empty-state checks, and a zero
frontend warning ratchet. Independent review returned P0 0 / P1 0 / P2 0. The
native no-bundle build produced SHA-256
`5C5FE3242DD2CBBCD2FD738F91497DEAD80D4ACD5761F26780340A6B36AAC3F6`.
The sole exact-path PID 63996 was responsive and its 1920x1032 window was verified
maximized before UI operations. Native QA created Bank 2 and Bank 3 from their
preselected defaults and one Scene in each of Bank 1/2/3, confirming equal widths,
absence of redundant empty copy, and purple/pink/green Bank-to-Scene colour
agreement.

This does not close output acceptance. The later OutputControl v2 checkpoint
removed the human-keyboard challenge and separate `Both` Arm consent. Verify the
one-click local Enable path, all five stable-identity fullscreen targets, and the
three-screen editor + LED panel + projector operating state.

## 19. 2026-08-21 Windows durable-output and release-evidence closure

The Windows release gate now treats an `-rc.N` version as a release-candidate
operation which requires the explicit candidate mode and a schema-validated
evidence manifest. The manifest is bound to the exact tag, HEAD, prior version,
clean worktree, artifacts, and updater identity. Executable metadata and the
runtime updater identity are inspected from one uniquely materialized copy of the
already hashed bytes, closing the artifact-path time-of-check/time-of-use gap.
The updater signature proof uses a real Tauri signer fixture and verifies the
Ed25519/minisign payload, key id, and trusted comment; payload, signature, and key
tampering are negative-tested.

Output Lease R4 terminal and pending results are now durably journaled across
restart without restoring lease authority. Take Over performs terminal lookup or
durable prepare before any physical stop/project publication, records the terminal
receipt only after acknowledged publication, and blocks in-doubt replay before a
physical callback. The journal is atomically written, bounded to 8 MiB, rejects
unknown fields, and semantically validates nested receipt/origin/pending records.
Corrupt, oversized, or unwritable state fails closed.

Measured evidence: the complete no-default Windows Rust suite passed 806 tests
with 5 intentionally ignored hardware/long-running tests and no failures; the
subsequent no-default `cargo check` emitted zero first-party warnings. Release
self-tests passed 65 assertion groups, normal release metadata passed, the
frontend warning ratchet remained zero, and formatting, syntax, and diff checks
passed. Independent read-only review returned P0 0, P1 0, and code P2 0.

Immediately before the native build, exact-path inspection found zero running
instances. `pnpm --dir app tauri build --no-bundle` completed in 1m53s. The exact
release executable is 53,442,560 bytes with SHA-256
`E28FF0260A9A2781BA6F072057C67DB18B53BA1139CD3065497BF3FB28D58ABF`,
FileVersion/ProductVersion `1.2.0-alpha.1`, and a successful exact runtime updater
diagnostic for the beta channel. It was launched as exact-path PID 83004; exactly
one responsive `Syndocal` window was returned. Computer Use confirmed it was
restored, selected Maximize through the native window menu, and recaptured the
sole 1920x1032 maximized window before further UI work.

This closes that historical Windows code/release-evidence tranche, not current
output acceptance. Do not run the retired Raw Input/six-digit/Enter challenge.
The current remaining actions are one-click exact-`Both` Enable, five detected
display windows with exact stable identities, and the retained editor + LED panel
+ projector operating state. Dangerous advanced operations use a parented Windows
warning dialog; normal Enable does not.

## 20. 2026-08-22 Windows expired-lease Add Display checkpoint

The normal Add Display route no longer depends on the public lease query that
truthfully hides naturally expired authority. A separate local-window-bound,
read-only query exposes one exact authority as active, naturally expired but
recoverable, or already orphaned. It fails closed for zero, multiple, foreign,
wrong-incarnation, wrong-project, split, subset, or otherwise ambiguous overlap,
and does not mutate lease generation, audit, or durable state. Add then issues one
canonical `add_display_output_v2` request; the private durable candidate either
authorizes the active exact-Both authority or recovers the exact expired/orphaned
authority and publishes the display output atomically. It does not run Enable as
a separate preliminary request and does not broaden public Acquire/Recover.

The former UI symptom was a false freeze: the inspected native process remained
responsive while the active lease had passed its 60-second TTL and the Add error
was rendered only in the global status area. Pending, success, and failure are now
adjacent to Add, and the monitor selector plus Add action are single-flight while
the request is pending.

Frozen pre-native evidence is control-plane 61/61, output-lease 46/46, dedicated
natural-expiry query-to-Add recovery 1/1, canonical four-display backend driver
1/1, frontend invoke inventory 407, localization 3538/3538, frontend build and
focused contracts PASS, no-default Rust warnings zero, frontend first/third-party
warnings zero, and formatting/syntax/diff checks PASS. The driver uses the same
canonical Tauri request schema and production durable/lease/engine/project path;
native shell creation alone is injected. Repeated validation must use this path,
with Computer Use reserved for one final maximized release-app acceptance.

This is not a completion claim. The authoritative checklist is still 13/79
(16.5%). A fresh `tauri build --no-bundle`, exactly one responsive maximized
editor window, default avoidance of that editor display, and four real sub-display
windows are unverified (0/4) and remain the next stop condition.

## 21. 2026-08-22 current-source native Add Display hang

The former false-freeze conclusion is superseded for the current source. Exact
pre-build process inspection found zero instances of this checkout's executable;
`pnpm --dir app tauri build --no-bundle` passed in 1m59s. The resulting
54,498,816-byte release executable has SHA-256
`30DD06DBD3642B6CE3D31E403FCDB6AA9A35D9D419929B4D744A837D3509C06E`.
It launched as the sole exact-path process, was responsive, and was maximized
before QA.

Enable succeeded. After the exact-Both lease passed its 60-second TTL, Add used
the default non-editor `\\.\DISPLAY5` 1920x1080 target and the parented Japanese
warning returned Yes. No output window appeared; Add remained pending; PID 81160
became Not Responding. Durable evidence contains terminal request 1 (`Acquired`)
and request 2 `Pending` with no terminal for the current renderer. The exact-path
process was stopped after evidence capture.

The blocking boundary is now a release stop. After durable prepare, Add holds
lifecycle, external-admission, project-coordinator, ownership-transition, and
lease-registry guards through native/GPU/first-frame work. At approximately ten
seconds the recovery timer invokes synchronous `get_project_checkpoint_bundle`
on the Tauri event loop and waits for the held coordinator. Completion requires:

- durable Pending before external effects;
- all unpublished window/GPU/first-frame preparation outside long-lived project,
  transition, lease, lifecycle, and owner guards;
- a short final boundary that revalidates owner, project, monitor identity,
  output/safety fence, lease owner/resources/generation, and S0 before publication;
- async/off-event-loop checkpoint capture with bounded contention behavior;
- SafeAbort only after acknowledged full cleanup, otherwise retained in-doubt
  Pending and zero replay;
- a production-path stalled-phase regression proving checkpoint/authority/UI
  liveness and stale-candidate rejection;
- a new exact-path release build and one maximized native session producing four
  real sub-display windows while the editor target remains avoided by default.

Repeated proof must use the canonical backend transaction path; Computer Use is
limited to the final physical placement/modal acceptance. Native display progress
is 0/4. The authoritative product checklist remains 13/79 (16.5%); no completion,
commit, or push is permitted at this stop.

## 22. 2026-08-22 phased Add Display Windows acceptance

The stop in section 21 is closed for the frozen phased implementation. The
production Tauri wrapper and deterministic tests use one full canonical Add
core, including durable Pending, monitor and authority capture, deadline-bounded
unpublished native/GPU preparation, exact final revalidation, engine ACK,
project/lease/worker/metrics publication, terminal record, and classified
cleanup. Independent read-only review reports P0 0 / P1 0.

The final no-bundle release build passed after exact-path process count zero.
Executable SHA-256 is
`C88A1DE53812DD773FE4AE28079977202104F33F63B67054119DD338C1EF2BAA`.
PID 56460 provided exactly one maximized responsive 1920x1032 editor window.
Enable remained responsive beyond the ten-second recovery interval. Canonical
v2 Add then created four real responsive native windows on the four non-editor
targets DISPLAY5, DISPLAY6, DISPLAY1, and DISPLAY3. The final process window set
is one editor plus four output windows, with no output on the editor target.

The current renderer's durable requests 1 through 5 are all terminal, its
origin has `replay_guard=false`, and it has no Pending. Historical Pending rows
from two older renderer origins remain fail-closed evidence only. The remaining
post-commit message P2 is now closed: terminal Add success is fixed before the
refresh, the two exact transient transaction/publication messages clear only
after refresh settles, unknown refresh failure reports `added; refresh pending`,
and mutation failure remains an adjacent Add error. The executable checker runs
the production classifier and the success/stale/unknown/mutation-failure matrix.

The final P2 rebuild used the same production code with unminified Vite assets
after Windows Defender falsely quarantined Tauri's generated minified JS; no
Defender exclusion or protection change was made. The successful no-bundle
release executable is 54,587,904 bytes with SHA-256
`977D3D63A60E77ACE9E6A6E1CA91652EC0356E8389367921DAB7032894DA0DCB`.
PID 87060 was the only exact-path process and supplied exactly one responsive,
maximized 1920x1032 `Syndocal` window after a greater-than-ten-second wait.

The Windows native display acceptance is 4/4 and this bounded tranche is 100%.
The product checklist remains 13/79 (16.5%). Continue the roadmap dependency
order; do not convert this acceptance into a whole-product completion claim.

## 23. 2026-08-22 E1 generic project-transaction acceptance

E1 is accepted for the Windows-first train. Generic Begin now carries a client
operation ID, strict schema and canonical shape, the exact backend-issued window
and owner incarnation, and the current project epoch/revision/hash. Pending,
Committed, and Cancelled receipts can be queried, adopted, and acknowledged;
same-ID shape changes, stale delayed Commit/Cancel, live-pane stealing, and
same-label owner ABA fail closed. Renderer retirement cancels only its exact
pending transaction, preserves one `Interrupted:` history entry when a partial
mutation exists, emits no history for no-change retirement, and keeps retryable
owner state when retirement cannot complete. Retired owner bindings are bounded
to 1024 and reject capacity overflow rather than reopening replay.

The frozen implementation passed the project-transaction checker, exact
frontend invoke inventory (410), 4/4 focused `project_transaction` Rust tests,
the focused partial-cancel Undo regression, TypeScript/Vite build, localization
3538/3538, empty-state, release metadata, Rust format, Node syntax, and diff
checks. Independent adversarial review returned P0 0 / P1 0 / P2 0. First-party
warnings were zero for the no-default Rust check, the default-feature Rust check,
the release native build, and the frontend warning ratchet. The structured
all-target warning command was also attempted, but its controlled child
environment could not select the absolute MSVC linker and exited during build
scripts; that environmental failure is not recorded as warning evidence.

Immediately before `pnpm --dir app tauri build --no-bundle`, the exact-checkout
running process count was zero. The build passed in 2m29s and produced the
54,958,080-byte executable with SHA-256
`0AACEA71AC666329A56DFBD57515650621E68DCD4D1B54E3ABA37918636C818C`.
It launched as exactly one responsive exact-path `Syndocal` process/window and
was explicitly maximized to 1920x1032 before the final capture. E1 changes no
visible interaction, so no repeated Computer Use mutation sequence was run.

The authoritative checklist is now 14/79 (17.7%). This is an E1-only acceptance,
not whole-product or cross-platform completion. Continue next with E2 authority
bundle/generation consistency; E3 durability and E4 Save/Save As remain open.

## 24. 2026-08-22 alpha.4 Timeline cue-audio integration checkpoint

The distributed development train is now `1.2.0-alpha.4`. This ordinal advances
the product metadata for the next intentionally distributed build; it does not
promote the current source to beta, RC, or release and does not change any
project, command, API, ABI, or asset schema version.

The sample-frame scheduler is independently frozen and reviewed. The exact
protocol hash is
`48FF684AFD66680DC97F1AB5447F5FAFF920E63A5609D9ABF1FC6F6CF502BD1B`, and the
exact Engine hash is
`0031D9EC1AB5004779A9C28A2665DF2BC14D6122B1FAE32B40DFCB15D5735F66`.
The full Engine gate passed 805/805 with zero failures, two ignored tests, and
one explicitly known filtered case; focused protocol/documentation gates and
the first-party warning gate were also zero-warning. The scheduler covers exact
sample-frame click/Guide ordering, variable meters, MTC/DJ discontinuities,
musical looping, and the pinned two-song `Trans` targets.

The final independently accepted native integration hashes are main
`E69A8989E7AE0D030C7AB3FE0AA31B2D36075CB22FADA0382EDDFE99D5214750`, cue core
`D6C1A18FD09E98AB7CA849EE439AFEF93C67C700E42466CB5B4C6854F90C1927`, and DVC
`B81913413A4796A37B4F6FE5A146CB75114AB20F0BE1A74B700A4A3C226F3546`, with
P0/P1/P2 all zero. Backend Cue Audio 41/41, DVC import 105/105, protocol 143/143,
frontend TypeScript/build, 411 invokes, 3541/3541 localization, focused Cue
runtime/browser, the four-viewport Timeline browser matrix, and Windows
default/release warning ratchets are green with zero first-party warnings.
After an exact-path process count of zero, the no-bundle native build passed in
2m37s and produced a 56,342,016-byte `1.2.0-alpha.4` executable with SHA-256
`AB98EA14F8439E23CC2E3BD80A4041C2B9CC82244A68B6938A3E9B2515A87297`. Exactly one
responsive `Syndocal` window was maximized and Edit > Timeline displayed Click,
Guide, Follow waiting, the shared lanes, and source shelf without a fault. No
project state was changed; audible playback, the final three-screen pass, and
fixture/serial-DMX/audio hardware acceptance remain required.

## 25. 2026-08-22 alpha.5 E2/D1 ledger and E3 automated checkpoint

The active distributed-development ordinal is `1.2.0-alpha.5`. This is a
product-version-only advance for a new artifact; it does not change project,
command, API, ABI, recovery-storage, or journal schema versions.

E2 and D1 are accepted from the independently reviewed alpha.3 production
implementation and the current rerun evidence. E2 applies authority bundles only
through the production event/reply/poll/fallback orchestration and preserves exact
authority/disposition generations. D1 keeps cache listing read-only and the custom
fixture preview route pure; Patch remains the mutation boundary. Their checked
Phase 1 boxes advanced the Windows denominator from 14/71 to 16/71. The native
E3 acceptance below advances it to 17/71.

E3 automated code evidence is independently green with P0/P1/P2 all zero. App and
the driver share the same v3 publication, startup, and intent-consumer functions;
the registered Rust load and acknowledgement commands share the same outer
lifecycle service used by the process-boundary test. The Rust proof observes
`durable -> retire -> publish -> commit -> event`, while an injected retirement
failure permits only the conservative durable recovery invalidation and produces
no engine publication, project-identity commit, event, or admissible ACK. Reply
loss, renderer/native restart interpretation, delayed ACK, competing C, duplicate
delivery, and CleanSave retirement are covered without mirrored state machines.

Current automated gates pass: E3 production driver, project authority/transaction
and storage checks, TypeScript, Vite build, exact 411 frontend invokes, 3541/3541
localization, Rust E3 1/1, project-recovery authority 2/2 and recovery-authority
3/3, no-default Cargo check, Rust format, diff-check, and frontend warning ratchet
with zero first-party and total warnings. At that automated checkpoint, E3 remained
unchecked pending native acceptance of the real Tauri dispatch and localStorage at
both crash boundaries: B published with its reply lost, and ACK durable before
renderer cleanup. The final operation also had to build and launch the alpha.5
release executable and record its exact hash/window evidence; the accepted result
is recorded below.

The first alpha.5 native repair acceptance is now recorded, without closing E3.
Immediately before the build, the only process whose resolved executable path was
exactly this checkout's `target/release/syndocal.exe` (PID `113004`) was terminated;
the exact-path process count then reached zero. The first build attempt stopped in
`ffmpeg-sys-next` before product compilation because the existing shared SDK had
not been exposed as `FFMPEG_DIR` and `pkg-config` was unavailable. Re-running
`pnpm --dir app tauri build --no-bundle` with the installed LGPL shared SDK root
as `FFMPEG_DIR`, its `bin` on `PATH`, and the installed LLVM directory as
`LIBCLANG_PATH` succeeded in 2m34s. The resulting
`target/release/syndocal.exe` reports ProductVersion/FileVersion
`1.2.0-alpha.5` and SHA-256
`F318FAEBEFBA88B03EC179DF34395E34F5BD8D960D37C2C579F8A209BD4BADC5`.
PID `54456` exposed exactly one responsive `Syndocal` window; the native titlebar
reported `Restore`, proving it was maximized before input. In that verified window,
Edit > Lighting created `Native QA Group` and reported
`Created fixture group Native QA Group.`; the former self-rejection
`Project transaction is active; retry after Display output publication` did not
occur. This closes the native regression for the renderer-ticketed fixture-group
Begin/commit route, but not the two E3 crash boundaries.

The crash-boundary preflight found that journal polling alone could not prove either
"durable B before command reply" or "durable ACK before renderer cleanup". An
ordinary-off acceptance helper was therefore added and independently reviewed at
P0/P1/P2 zero. Its final frozen hashes are main
`5014DEA37D6788FB6EECFAA4A5A7C9FF59FD83C593F93EC6D27F5EBE83D5AF23` and helper
`17CF98F17095EE89559CA26B03772A09AE5C85CE60F158D6712434A643DDADC8`;
`e3_` passed 6/6, no-default Cargo check reported zero first-party warnings, and
format/diff checks passed. The helper accepts only a fixed local control directory
under the executable, rejects UNC/device/outside/parent/reparse/non-fixed-drive
paths before remote probing, requires an exact resume record, and is a zero-I/O
no-op when its environment is unset.

Native E3 acceptance then used the rebuilt 56,461,824-byte alpha.5 executable,
ProductVersion/FileVersion `1.2.0-alpha.5`, SHA-256
`97C21F367A46375A2B6010CC6E9307E7D853CEC4EAF420150A06CA47AD9D4182`.
The release build passed in 1m42s after the exact-checkout process count was verified
as zero. In the first pass, PID `94632` paused after durable RecoveryPublication
serial 44 and before event/reply for request
`69fb623e-7640-4cda-b7cf-261e70f0685d`, checkpoint
`3a7e24b723a38dd7cffd478b67fc87c78a9df6fba09eb9e1e2825a2a6d4d22cd`.
The exact process was force-terminated; journal serial/hash and the WebView LevelDB
intent remained. In the second pass, PID `67752` published a fresh request
`da5547eb-fdd5-4fcc-84f9-3f658aae706f` at serial 45, resumed from the exact trace,
then paused after durable RecoveryAcknowledged serial 46 and before command reply/
renderer cleanup. Its journal SHA-256 was
`22E329110C96997CFEC893CFE6EF48D5F9F77A5A453C9DE64FC91ABACAF650F9`;
the exact process was again force-terminated with the matching LevelDB intent still
present. A final hook-free launch produced exactly one responsive, maximized
`Syndocal` window at PID `44376`; after startup journal serial 46 and its SHA-256
remained unchanged. The recovery offer remained, as required for the acknowledged
checkpoint until a later coherent CleanSave. E3 is accepted; E4 is now the next
Phase 1 tranche, and AI4 remains blocked by the dependency order.

Immediately before committing this checkpoint, the warning ratchet rejected five
new `too_many_arguments` suppressions in renderer-ticketed fixture helpers. They
were replaced with one internal context object without changing Tauri command
signatures or wire schemas. The executable lock-order audit was strengthened to
prove direct mutators, Begin/Commit/Cancel delegation, the shared renderer owner
gate, and all nine exact wrapper/helper/command bindings. Independent review
returned P0/P1/P2 zero; the targeted audit, E3 6/6, recovered fixture 1/1,
project-transaction 4/4, no-default Cargo check, format/diff, and the frontend
warning ratchet all passed with zero first-party warnings and zero new suppressions.
Because this behavior-preserving native refactor changed `main.rs`, a final release
build was run from the checkpoint source. It passed in 1m49s and produced the
56,470,016-byte executable SHA-256
`8FDFD0EFB8D71D49BD1A137A4AA68F440F098B2C7B3BA253461F5C2AC14D6F19`,
ProductVersion/FileVersion `1.2.0-alpha.5`. PID `113948` provided exactly one
responsive, maximized `Syndocal` window. The crash-boundary evidence above remains
bound to the explicitly identified `97C21F...` acceptance executable; the final
build proves the reviewed checkpoint source still satisfies the native launch gate.

## 26. 2026-08-23 alpha.6 E4 automated checkpoint

The distributed-development ordinal advances to `1.2.0-alpha.6`. E4 adds a
versioned project-publication request, pending state, terminal receipt, exact query,
acknowledgement, abandonment, and restart-owner adoption contract. This is an
additive command and recovery-journal compatibility boundary; it is not hidden by
the product-version advance. Save, Save As, user template, manual/autosave backup,
and update preflight now share one durable reservation/freshness/publication lane.

The fixed backend candidate at main SHA-256
`59AC69E8D91E2D8EA2CEBFCA9374425D048F47218AFEE8B2B470346F2F356AAA` and
control-plane SHA-256
`3B73847CAF4F9FC39F84F44D2109BEB5224BA349A10560ECF3232B463CB9BA75`
received independent P0/P1/P2-zero review. The frontend adoption/controller
candidate also received independent P0/P1/P2-zero review. Automated evidence is
green: project-publication 15/15, updater final-fence 1/1, restart reconciliation,
CleanSave ownership, same-millisecond backup uniqueness, control-plane 25/25,
the production E4 controller/checker, exact 415 frontend invokes, TypeScript/Vite,
no-default Cargo check, format/diff checks, and zero first-party warnings in every
modified configuration run.

The synchronized alpha.6 native acceptance is now complete. After stopping only
the exact checkout executable, `pnpm --dir app tauri build --no-bundle` passed in
1m50s and produced the 57,085,440-byte executable SHA-256
`687E7BFD8B9A74C7A3F91493C2FFAF56D97B60B32B0D0EDF6D45C2209EA563F7`, with
ProductVersion/FileVersion `1.2.0-alpha.6`. Exactly one responsive, maximized
`Syndocal` window was present.

The first alpha.6 UI pass correctly exposed an invalid native User Template
terminal DTO. Commit `d707872` restricts project authority to successful
Save/Save As receipts; User Template and Backup receipts now remain non-project
receipts through direct and restart-query paths. The strengthened Rust/frontend
matrix and independent review are green at P0/P1/P2 zero. The rebuilt executable
then passed scratch Save, Save As, User Template, and autosave Backup. Evidence is
rooted at `C:\TEMP\syndocal-e4-qa-alpha6`: `qa-final-save-as.sdc`,
`qa-template-fixed.sdctemplate`, and managed backup
`backup-1787421898035.json` (SHA-256
`01CCCBC4F99ECCC038FB80788ECBA774CDC2F1E6E1E9DBA65C7425E8A98C0EC3`), whose
source is `qa-final-save-as.sdc` and whose payload contains the post-save
`QA Backup` fixture group.

E4 is accepted for the current Windows tranche. The accepted Windows denominator
is now 18/71 (25.4%). This does not complete the product; later roadmap and
current-PC hardware/ASIO/soak gates remain open.

## 27. 2026-08-23 alpha.7 D2/UI/Guide integration checkpoint

The next intentionally distributed development artifact advances to
`1.2.0-alpha.7`; alpha.6 artifact bytes remain immutable. This is a product-version
advance only. It does not change any project, template, command, API, ABI,
checkpoint, cache, or audio-asset schema version.

D2 is accepted from the production PATCH/GDTF Repair transaction path. Whole-batch
profile resolution and validation complete before publication, allocator and
runtime state roll back on definitive failure, admitted ACK uncertainty remains
fail-closed, and the exact typed command result survives reply loss so lifecycle
retirement cannot rewrite a published result as Interrupted history. The final
fixed backend hashes are main
`5B91759B9B4789AF17B0376220F094B08603D0AD271DEF9B5CD67AA46103215D`, Engine
`03685241C9FC35B12F7FDA6A880ADEB64D3EF092F60B63717E8325ACEC0C0C6F`, and
control plane
`A0864AB959688F5B857F5F49BF985F82F64036F145C3445FDCFA113E9B480750`.
Independent review returned P0/P1/P2 zero. Focused evidence passed D2 9/9,
project-transaction 4/4, control-plane 25/25, Engine fixture PATCH 10/10, Engine
Repair 5/5, exact frontend invoke inventory 415, the D2 controller, backend
operator-contract checks, Rust format/diff checks, and the no-default Rust check
with zero first-party warnings. D2 advances the accepted Windows denominator to
19/71 (26.8%).

The integrated UI evidence is also green without closing a hardware or whole-UI
row: Lighting, Video, and Timeline geometry plus screenshots passed at 1920x1080,
1366x768, 860x520, and 1280x720; the Timeline operator matrix passed five
viewports; and the Timeline Sources lower pane retained zero outer/body scroll.
The pinned click/Guide perceptual check passed its 150 ms boundary. These are
automated/browser/render and deterministic audio proofs, not native release-app,
audible-device, editor-plus-two-display, DMX, MTC/DJ, ASIO, or soak acceptance.

No alpha.7 native build or GUI operation is claimed at this checkpoint. The next
stop is the root-owned synchronized `pnpm --dir app tauri build --no-bundle`,
followed by exactly one responsive maximized `Syndocal` window and the remaining
current-PC hardware/UI acceptance. Until that succeeds, alpha.7 is an automated
integration checkpoint only.

The version checkpoint gates passed: `pnpm --dir app run check:release` reported
exact synchronized alpha.7 metadata; `cargo check -p syndocal --locked --bin
syndocal --no-default-features` completed with zero first-party warnings; the
`frontend-typescript-vite-windows` warning ratchet reported baseline/current
0/0 total and first-party warnings; and `git diff --check` passed with only Git's
line-ending notices.

## 28. 2026-08-23 alpha.7 Control upper-workspace native/UI closure

The Control upper-workspace regression was traced to layout ownership rather than
an intentional compact-mode change. Lighting mounted its populated Scene Matrix
behind a stale selector; Video placed the Media Library outside the intended
grid/flow; and Timeline split its header and portalled upper content across
incompatible rows. The Timeline Sources shelf then inherited an undersized header
track and outer overflow, producing the clipped labels and nested scroll reported
from the real window. The repair restores the populated upper owner for Lighting,
puts Video's Media Library back in the upper grid with internal body scrolling,
gives Timeline a fixed header plus remaining-height content row, and keeps Sources'
outer shell contained while only its body may scroll. Tools/Live Mixer disclosure,
expanded Timeline, and ordered Escape unwinding were retained without shrinking
shared typography, controls, spacing, icons, or hit targets.

The dedicated browser gate and all 28 captured states passed at exactly
3840x2160, 2560x1440, 1920x1080, and 1280x720. Those four sizes are the current
Control upper-workspace browser acceptance matrix. 960x640 remains the configured
product minimum only; 860x520 and 1366x768 are historical supplemental cases and
are not substituted for this matrix. Browser geometry and screenshots do not by
themselves establish native acceptance.

The synchronized alpha.7 native build subsequently passed. After the exact
checkout process pre-stop, `pnpm --dir app tauri build --no-bundle` passed in
2m46s and produced
`target/release/syndocal.exe`, SHA-256
`00253F26A8D3A933172D7B07923E430B455CA18F98E439CE45C5B62405F86EF4`.
Launch inspection found exactly one responsive `Syndocal` main window at PID
`100260` from that exact executable and it was maximized before UI interaction.
At the 1920 desktop
class (1920x1032 maximized work area), native screenshots verify populated
Lighting and Video upper workspaces, the Video Import disclosure, Timeline lanes,
the full-height Sources shelf, contained Tools, expanded Timeline, and the ordered
Escape path back to the lower panes. A maximized 2048x1104 pass is supplemental.
Exact native 2560x1440 and 1280x720 placement was not accepted because the window
could not be moved to those active displays through the verified safe automation
route; 3840x2160 was connected but not active as a desktop mode. These three native
sizes therefore remain explicitly unverified even though their browser rows pass.

The deterministic click/Guide check also passes the requested perceptual lead:
the Guide onset is the authored click target minus 7,200 frames, exactly 150 ms at
48 kHz. This is sample-frame evidence, not audible-device acceptance. D2 remains
complete at 19/71, and the Windows configurations exercised by this checkpoint
report zero first-party warnings. The supplemental `check:edit-live` gate still
has two pre-existing, unrelated failures (scene-identity badge color and an old
<=86 px mode-tab compactness expectation); neither is caused by the Control upper
workspace repair, and the current UI was not shrunk to satisfy that stale rule.

This checkpoint closes the reported Control upper-workspace/native 1920-class
regression only. Physical multi-display VJ output, DMX, MIDI, DJ Link plus Stream
Deck Pedal, audible click/Guide routing, the remaining ASIO matrix/licensing/fault
rows, and the integrated long soak remain mandatory before product completion.

## 29. 2026-08-23 intentional pause checkpoint

Work stopped before alpha.8/native/hardware execution. The then-current stop state,
partial D3 freeze, statically approved Tauri mitigation, DJ transport freeze, and
resume order were recorded in `qa/SYNDOCAL_PAUSE_HANDOFF_2026-08-23.md`. That file
is historical. The later post-alpha.10 snapshot preserves the next exact evidence
boundary, but its requested pause was rescinded before promotion. The accepted
denominator at this historical checkpoint remained 19/71 (26.8%); no paused item
advanced it.

## 30. 2026-08-24 pre-alpha.8 D3 and source-gate closure

The strict D3 sequence is complete on `codex/syndocal-v1.2` at base HEAD
`23f350c366ede2fdffcfbf3232e18112eada51ea`. Independent final fixed-hash
review returned P0 0 / P1 0 / release-blocking P2 0. Full suites passed at Engine
822/2, I/O 148/1, and Syndocal no-default 951/5, with zero failures and zero
first-party warnings. Final source SHA-256 values are main
`2CC94D3E5307E6BBC026F1EC815ABC02BFF47E987D62526446AFD1059984E6C7`, Engine
`F2DCE4D4F6E4B9CA4E15DB722608461D9D0ADBD9948FB41D5796FD4B94E1B2F7`, and I/O
`E552FEA70D017BBAE40B534A6D854BE1C1B20AFFE111B9E733E0C19FA7A1E8B4`.

The Tauri adapter fail-closes Raw/invalid invoke bodies and preserves absent/null
optional semantics across all 51 flat commands (39 common, 11 video, one repair);
six focused tests and its own P0/P1/P2-zero fixed-hash review passed. The hardened
warning ratchet also passed review and all exercised warning counts are 0/0. Its
library/test hashes are
`55D2388C3EA244F3C58B207E651E56F33DB10B6E76BD404A2943497E384926AA` and
`3391F2F149ADCED3B5641E48E326680686B8D243389EAFC7E2DCC3799AEA7E55`.

The frozen frontend/static chain passed routing 133/29/33/402, 417 invokes,
backend 478/311/133, project authority/transaction, E3/E4, output ownership and
runtime, the five-viewport Scene Matrix/strip contract, DVC DMX 35 assertions,
localization 3556/3556, and worktree/cached diff checks. The Scene Matrix and DVC
DMX corrections both received fixed-hash reviews without a release-blocking finding.
The companion DJ Agent is pushed at `Beta` commit
`6c4f4328a6866d9d48022bd8ee20a7887c9de851`, with 54 tests, 16 syntax checks,
zero warnings, and packaged `server.exe` SHA-256
`339ECF6E82EB463F55977F63A137CB0CB52886CD7E2874E87F5AD4724234377B`.

This checks D3 only. D4 and the Phase 1 exit remain open, and no native, physical
DJ, display/output, DMX/MIDI, audible-device, ASIO, or soak gate is inferred. The
accepted denominator stays exactly 19/71 (26.8%). Product metadata remains
`1.2.0-alpha.7`; the intended source manifest is
`qa/artifacts/source-freeze/2026-08-24-alpha7-pre-alpha8-source-freeze.sha256`.
Next is synchronized alpha.8 metadata and `check:release`, followed by native
build/launch/WER/reload and the current-PC physical acceptance sequence.

## 31. 2026-08-24 Step 6 alpha.8 product-metadata checkpoint

Step 6 advances only the current product train from alpha.7 to
`1.2.0-alpha.8`. The 20 authoritative synchronization points passed an exact
20/20 audit: workspace and first-party lock packages, frontend/Tauri metadata,
release/CI/macOS packaging declarations, current README names, and the three
current-train document lines. Historical alpha.7 build and source-freeze evidence
remains unchanged.

`pnpm --dir app run check:release`, `pnpm --dir app run
check:tauri-build-wrapper`, and `cargo metadata --locked --no-deps` passed; the
locked metadata reports all eight first-party packages at `1.2.0-alpha.8`. The
Windows frontend TypeScript/Vite warning ratchet rebuilt the frontend with
total/first-party/third-party warnings 0/0/0 and output-marker coverage 3/3.

This product-version checkpoint changes no project, template, cache,
control-plane, command, API, ABI, or audio-asset schema version. It does not claim
an alpha.8 native build, launch, WER/reload result, physical hardware row, ASIO/DJ
acceptance, or soak; the accepted denominator remains 19/71 (26.8%).

## 32. 2026-08-24 current-source alpha.8 native and display checkpoint

The synchronized train is now `1.2.0-alpha.8` on
`codex/syndocal-v1.2` at pre-commit base HEAD
`23f350c366ede2fdffcfbf3232e18112eada51ea`. The current-source frontend
validation log `target/qa/alpha8-current-source-final-gates-20260824-114918`
contains 25/25 passing gates. The current CSS blob, upper-workspace checker blob,
and Scene Matrix checker SHA-256 are respectively
`4974a2f828b8b8bd1c9fbe43390d97d5d6702179`,
`73a43ecfb2c6f10c07fb638f84f50375de5213a0`, and
`F164CD5B5C6C130E1D27B21C6A04CB1C361CEE3346F08FA9DFF77DE522C5FE11`.
Independent fixed-hash review is P0=0/P1=0/release-blocking P2=0, with
exercised warning counts total/first-party/third-party 0/0/0.

The authoritative staged-source inventory is
`qa/artifacts/source-freeze/2026-08-24-alpha8-current-source-freeze.sha256`.
It contains 103 payload records and excludes its own manifest envelope from the
payload to avoid recursive self-hashing. The alpha.7 pre-alpha.8 manifest is
historical evidence only, not current alpha.8 source authority.

The exact-path pre-stop was completed before the required
`pnpm --dir app tauri build --no-bundle`; the command passed from the Visual
Studio Developer Shell using MSVC 14.43.34808 and Windows SDK 10.0.26100.0.
The current executable is 57,491,456 bytes, SHA-256
`627BE88032774C7FA0A4C3CD3510A7BFB52E8ED0E76884ADD414B9BFD101F459`, with
FileVersion/ProductVersion `1.2.0-alpha.8`; its 19,582,976-byte PDB has SHA-256
`5548E4F4B2C3CBB38F1881AAA6C9299AE42211616A8A05E9189C3019838F56AB`. Launch
verification found exactly one responsive maximized Syndocal window.

The fresh current-source reload JSON
`qa/artifacts/native-physical-acceptance/2026-08-24-alpha8-current-source-final/native-reload-stress-100-current-source.json`
has SHA-256
`6D12AA14371B1C837DF67DDE80AB44CB1C6B1329F093567B81524E35076FB621` and records
100/100 reloads, 100 unique origins, exit 0, total 40.1424 s, p50 407.3 ms,
p95 479 ms, max 530.1 ms, runtime/log issues 0, and WER Application Error and
Reliability deltas 0. The former `2026-08-24-alpha8-final` directory is retained
as pre-CSS historical evidence; current-source claims use
`2026-08-24-alpha8-current-source-final`.

The mandated native five-display pane route is complete via the supported
`open_pane_window` placement path: `D5 -> D2 -> D3 -> D1 -> D6`. Every pane was
maximized and each route step recorded zero document scroll. Measurements are
D5 monitor 1920x1080/scale 1.5/viewport 1280x650 at `{-2465,1731}`; D2
1920x1080/scale 1/viewport 1920x1009 at `{0,0}`; D3 real physical 3840x2160
4K/scale 1.5/viewport 2560x1370 at `{-3840,-429}`; D1 2560x1440/scale
1.25/viewport 2048x1082 at `{1920,-364}`; and D6 2560x720/scale 1/viewport
2560x649 at `{1598,1080}`.

Canonical evidence is
`qa/artifacts/native-physical-acceptance/2026-08-24-alpha8-current-source-final/native-display-route-current-source.json`,
SHA-256 `41F1D6E2528E7439657F8879F753255221E25F2DF0474139B56C1570E2C32C41`.
The route screenshots are D5 `37EDB56EA3F81BA014C23A96A1C78F32EEBB68C6B3E4C3D7C6692FD30E2FD0E1`,
D2 `0A037F7E6359FBC22C1FC16F297D9A3860157544777764679FABD2C194C20CFD`, D3-4K
`55E83F6AFC32A32DD9BD5AAB187DA659DC0221DB0EF82EAEAA65215327F591CD`, D1
`C22918826714D16E0E02DB5190AFB1E2724CEB387943A0D3FA4FABCDE5DD645E`, and D6
`2336172DB637B9E4F9D4906B3E7D6D781408CD0A4B11D18229974FC92E820340`.
The expanded D5 evidence `D5-timeline-tools-expanded-fixed.jpg` is
`B780CD4CCDA35CC8A8F1148A64B26C91065EB15E33D57A38FDCC04F8B0A3724D`; native
popup client/scroll is 345/345, nested surfaces are 335/335, and the deepest
44px target is inside the viewport/popup with hit/focus proof and Escape focus
return. The pane was closed; final verification found one exact responsive PID
123952, CDP page 1, and maximized D5 main viewport 1280x672.

All five detached Timeline pane screenshots still show
`Window 'pane-timeline' has no current project transaction owner registration`
six seconds after each pane opened. A same-time
`get_project_authority_bundle` read succeeds with epoch 0/revision 1, but that
does not prove transaction-owner registration or make the persisted status stale.
This evidence accepts placement, maximize, and containment only; it does not
accept transactional pane operation, warning-clean pane startup, or completed
owner registration. That P1 remains an alpha.9 boundary.

This closes native multi-display pane placement/containment only. It does not
claim display-output playback, fullscreen playback, GPU reset/recovery, hardware
DJ Link/rekordbox, Stream Deck Pedal/MIDI, ASIO, real DMX, fault/recovery
matrices, or the integrated one-hour soak. The accepted denominator is unchanged
at 19/71 (26.8%).

## 33. 2026-08-24 alpha.8 push and alpha.9 registration-order tranche

The alpha.8 current-source/native/display checkpoint was committed as
`ec9fca4887e079fa61950056d94aca5ab5d65da9` and pushed to
`origin/codex/syndocal-v1.2`. The post-push branch was exactly synchronized at
ahead/behind 0/0 with no staged or unstaged tracked files; 19 old evidence paths
remain intentionally untracked.

The next distributed development ordinal is synchronized at `1.2.0-alpha.9`
across 20/20 authoritative coordinates. Release metadata and locked Cargo metadata
pass. Alpha.9 first repairs the detached-pane transaction-owner startup order:
every WebView must register before owner-bound invocation, a failed registration
must remain fail-closed yet re-arm on a later operation without a tight loop, and
only the main window may consume startup or queued project opens. Completion
requires focused gates, the exact-path native release build, fresh native pane
evidence with no persisted owner-registration status, independent review, and
commit/push. Until then the accepted denominator remains 19/71 (26.8%).

## 34. 2026-08-24 alpha.9 registration-order closure and alpha.10 entry

Alpha.9 satisfies its source, automated, native-build, and fresh-pane proof
requirements. Each WebView now blocks owner-bound work on one sticky fail-closed
registration barrier; only later trusted activity can re-arm a failure, main-only
startup/open consumption remains exclusive, disposed completions are ignored,
and desired selection/audio work is serialized. Focused checkers cover concurrent
callers, rejected registration, recovery, disposal, and bootstrap ordering.

All focused release/routing/operator/transaction/runtime/workspace/localization/
format gates passed, ten focused Rust tests passed with the exact pinned MSVC
linker, and first-party plus third-party frontend warning counts were 0/0. The
required no-bundle native release build passed. Its EXE SHA-256 is
`BD4375D09EA09E099E6F24D74DC57B014E60F0C4C124401D1CBA6ACB1EE207FF`.
The privacy-safe native record is
`qa/artifacts/native-owner-registration/2026-08-24-alpha9/acceptance.json`.

Native acceptance used the real Workspaces controls for all seven detachable
panes and proved registration-before-selection, main reload without pane-owner
rotation, no registration status/error/crash, and final return to one responsive
maximized main window. Five displays including a 3840x2160 display were enumerated
only. The 19/71 (26.8%) denominator does not move because physical playback,
hardware, fault/recovery, ASIO, and soak rows remain unaccepted.

Alpha.10 starts with a separate P1: both Stage and Timeline detached currently
leave an empty lower main-window band. Completion requires collapsed outer and
inner grid tracks, upper-content expansion in restored/maximized/fullscreen
modes, usable Stage and Timeline rejoin controls, inert hidden splitters, saved
ratio restoration, focused browser proof, and required native geometry proof.
After each committed/pushed checkpoint, verified-regenerable stale build/cache
output must be inventoried and removed while current release/QA evidence remains
protected.

## 35. 2026-08-24 alpha.10 pane semantic correction — accepted

The alpha.10 entry paragraph above is **historical and superseded** as an
acceptance topology: its "upper-content expansion" wording did not require the
real Stage and Sources. Alpha.7 and alpha.8 remain immutable historical train
evidence. Alpha.10 was subsequently committed/pushed as
`5c7e19a72a97e20f5ece553594841103990a78a9`; its exact focused/native/source-
bound physical evidence and the transient requested-pause snapshot are indexed by
`qa/SYNDOCAL_POST_ALPHA10_PAUSE_HANDOFF_2026-08-24.md`.

Control/LIVE must render the real Timeline above, real Stage/Groups lower-left,
and Sources lower-right. With Timeline detached, only Timeline leaves and the
main window's real Stage/Groups plus Sources use its full height. With Stage
detached, only Stage/Groups leave and Timeline plus Sources remain. With both
detached, Sources alone fill the entire main workspace. A sparse Timeline
Preview or empty lower band is not an acceptable Stage replacement.

The final focused and native acceptance must prove both detachment orders
(Timeline then Stage; Stage then Timeline), both reverse rejoin orders, child
titlebar `X` closure, expand-before-detach and expand-before-rejoin, and outer
document/app scroll of zero in every state. Setup/Patch Groups remain available;
Setup, Edit, and Mixer remain non-regressed. The final native record is reserved
for maximized/restored/F11 classes and physical 3840x2160-at-150%-scaling
measurements. Those alpha.10 rows passed on the corrected committed source and
were independently reviewed; this does not close any of the 51 unchecked product
rows, ASIO, DJ peer artifact, distribution, or hardware boundaries below.

## 36. 2026-08-25 continuation after alpha.10

The operator explicitly rescinded the requested stop before its final authority
promotion and directed work to continue. No alpha.10 source or acceptance evidence
was rolled back. The pushed rules/evidence checkpoint is
`e9c6512e3a669eb3bd2d056a74ded102d77521d5`; the exact alpha.10 QA process was
stopped after capture, so any further native operation must relaunch only a newly
verified exact-checkout executable and obey the linker/native gate in `AGENTS.md`.

Resume the dependency order from the first unfinished accepted-boundary work: audit
and integrate the frozen D4 Stage transaction lane, independently audit the frozen
ASIO persistence lane without claiming real-device acceptance, then continue the
remaining 51 rows in section 6. The post-alpha.10 snapshot is evidence and an
exhaustive residual cross-check, not a prohibition on this explicit continuation.

## 37. 2026-08-25 alpha.11 D4 native acceptance

D4 Stage transaction software/native integration is accepted at
`1.2.0-alpha.11`. The focused/static evidence recorded earlier on this branch
remains binding; the native rows were closed on `codex/d4-stage-integration`
at committed checkpoint HEAD `63cf795d17846602419d63a007db9f3a95cfce7b`,
recorded before later hardening of the native acceptance tooling. The expanded
`app/scripts/check-native-window-acceptance.ps1` was committed at `a0c76c5` after
PowerShell 7 and Windows PowerShell 5.1 each passed 81/81 deterministic self-test
checks and an independent adversarial review. That source checkpoint is not a new
native acceptance run: the script's expanded restart/reload/pane-lifecycle matrix
remains to be exercised against the next accepted alpha.12 executable, and the D4
native evidence remains pinned to `63cf795d`.

The native gate resolved and pinned the exact VS2022 x64 linker
`C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`
(file version 14.44.35225.0, SHA-256
`1523A87532C2EB737DD7B7BCFC652CE5687A4F16048A45D1A5A0E8F6451AD49E`) before
Cargo/Tauri and exposed the complete shared SDK root as `FFMPEG_DIR`
(`C:\Users\kouty\AppData\Local\Microsoft\WinGet\Packages\Gyan.FFmpeg.Shared_Microsoft.Winget.Source_8wekyb3d8bbwe\ffmpeg-8.1.2-full_build-shared`).
`pnpm --dir app run check:warnings -- --configuration windows-native-release`
passed and internally ran `pnpm --dir app tauri build --no-bundle`
successfully with warning totals current/baseline total 0/0 (first-party 0,
third-party 0). The artifact is the 57,888,768-byte
`target/release/syndocal.exe`, SHA-256
`1B010C40242A5C7DD7A2797EAC1ECA2D31BCACE4455BA57C7F935611075B582B`,
ProductVersion/FileVersion `1.2.0-alpha.11`.

The exact executable launched as PID 117672 and, after integrated-state
cleanup, presented exactly one responsive top-level application window titled
`Syndocal`; the intended window was verified maximized before UI operations.
The active display was the real 4K `DISPLAY3`: 3840x2160 @ 240 Hz, Windows
scale 150%, bounds `(-3840,-429)`, work area 3840x2088, app client physical
3840x2088 / CSS viewport 2560x1392. With Timeline and Stage detached, the main
Sources surface expanded through the available workspace with no blank lower
reserved pane. Closing the detached Timeline reintegrated a single fully drawn
Timeline while the detached Stage remained separate with Sources below it.
Closing the detached Stage reintegrated Stage lower-left and Sources
lower-right beneath Timeline; `list_windows` then contained only one Syndocal
window, no duplicate Timeline appeared, and the integrated Stage was visibly
drawn. This directly resolves the operator-reported blank-space,
duplicate-Timeline, and missing-integrated-Stage concerns.

Manual D4 transaction QA ran in that verified maximized native 4K window. A
stage object labeled `D4 QA alpha11 20260825-0540` appeared on the Stage map
and panel. The Project menu offered `Undo Add Stage Object 1 step`; Undo
removed it completely and enabled Redo; Redo restored the exact label and map
object with status `Redid Add Stage Object`. The unique stage map preset
`D4 QA alpha11 preset 20260825-0550` was saved holding 1 stage object. The X
maximum changed 10.0 -> 10.1 through the native spin control (status
`Updated 2D stage map`, map re-rendered); applying the preset restored the X
maximum exactly to 10.0 and retained the object (status
`Applied stage map preset ... (1 object)`); the QA preset was then removed
(status `Removed stage map preset...`). The test-fixture transform A/B step
was not performed because the launched Untitled project had zero patched
fixtures; that fixture/hardware-specific behavior remains an explicitly
unverified boundary rather than a D4 stage-object/preset blocker.

This accepts the alpha.11 D4 software/native integration row only. It does not
claim whole-product completion, a beta/RC/tag/release, Windows ASIO completion,
or any physical/hardware acceptance; all device/network/soak boundaries in
sections 7-8 stay open. Next action: ASIO alpha12 authoring route repair/
verification plus the ongoing physical-device boundaries.

## 38. 2026-08-25 alpha.11 D4 integration into the active branch

The accepted D4 branch was staged for a no-fast-forward merge into
`codex/syndocal-v1.2`. Its pre-merge parents were
`3067720c253af06e9fb2c59a5831f4c38a2916dc` for the active branch and
`0bfc1c03451373f4cb34eebbfab4c5c257878fe0` for D4. The only conflicts were
the documentation files `RELEASE_STATUS.md` and
`qa/REKORDBOX_STREAM_DECK_PEDAL_ACCEPTANCE.md`; no source file conflicted. The
manual resolution retained the exact-linker alpha.10 checkpoint, the complete
alpha.11 D4 state, and the truthful historical/unbound v1.1.1 DJ artifact
boundary.

After resolution, release metadata and its 102-group self-test passed. Backend
inventories were 480/310/133; the D2 patch, project transaction, and authority
checks passed; frontend routing inventories were 133/29/30/407; the guarded
Tauri wrapper passed 156 assertions; and the production frontend build
transformed 272 modules. Under the exact VS2022 x64 linker gate, focused Cargo
tests passed engine D4 6/6, backend D4 7/7, and pane lifecycle 4/4 with zero
first-party warnings. Independent staged-diff review reported P0/P1/P2
`0/0/0`, verified no conflict markers or unmerged index entries, and confirmed
`git diff --cached --check` clean.

This integration proof does not replace the native gate: after the merge is
committed and pushed, the exact main-checkout alpha.11 executable must be
rebuilt, launched, and verified as exactly one responsive maximized Syndocal
window. ASIO, DJ-Link live LAN, and every other physical-device boundary remain
open until their dedicated acceptance runs.

## 39. 2026-08-25 alpha.11 active-branch native checkpoint

The reviewed merge was committed as
`b4a5b62ad48c7e3e58f78c66cf2914e0f53a46f5`, pushed to
`origin/codex/syndocal-v1.2`, and left at upstream divergence `0/0`. The exact
main-checkout native build then passed through
`pnpm --dir app tauri build --no-bundle`. Before Cargo, the wrapper printed the
required VS2022 Community 14.44 linker first and Git's linker second and pinned
the exact VS linker in `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`. The
frontend transformed 272 modules; the release profile finished in 3 minutes
16 seconds with zero first-party compiler warning lines.

The resulting `target/release/syndocal.exe` is 57,888,768 bytes,
ProductVersion/FileVersion `1.2.0-alpha.11`, SHA-256
`522074E96A235310C9D39D3200E2A9D9B5C68429F0160E92318B568A0FE5AEE2`.
It launched from the exact active checkout as PID 114780. Exactly one
targetable responsive Syndocal window existed and a fresh capture showed it
maximized over the 1920x1032 Windows work area. Section 37's committed D4
artifact remains the authoritative real-4K Stage/Timeline detachment evidence;
this main-checkout launch proves merge parity without overstating a new 4K run.

After the build, 36 independently reviewed obsolete/rebuildable targets (2,245
files, 1,184,114,966 logical bytes) were submitted by exact path to the Windows
Recycle Bin API and disappeared from their source paths. This set was limited
to one corrupt retired PDB, one obsolete
unpacked 1.0.0 MSI image, 33 old UI/browser scratch directories, and one
standalone OpenCode test-crate target. The post-operation audit found no
matching current-user Recycle Bin metadata, so recovery is unverified and must
not be relied on; no C: physical free-space recovery is claimed.
The active release, broad `target/debug`, current QA evidence, and active D4/
ASIO worktrees were preserved.

A second exact-path cleanup tranche then permanently removed five
independently audited rebuildable caches: `target/warning-capture`,
`target/vendor-wry-review`, `target/root-warning-review`, `target/asio-qa`, and
`target/wsl-node`. Their matched pre-delete inventory was 19,201 files and
5,654,458,899 logical bytes. All five paths were absent afterward; C: free
space increased by 5,410,566,144 measured bytes. The warning-log SHA-256 values,
irreversible deletion boundary, and retained ASIO SDK/LLVM/release protections
are recorded in the alpha.11 main integration checkpoint. This cleanup is a
capacity recovery result only and creates no runtime, warning, ASIO, DJ-Link,
or release acceptance claim. The execution-time inventory, deletion API, and
free-space measurements are operator-recorded because their raw terminal
transcript and deleted inputs are not retained; current re-verification proves
only source-path absence and protected-artifact presence.

The exact D4 stage-transaction build target was also deleted after an
independent audit and immediate revalidation: 15,257 files and 15,289,402,815
logical bytes. The source worktree stayed clean on its pushed branch and the
branch's six main-external commits remain preserved. The path was absent after
deletion and C: free space increased by 13,469,499,392 measured bytes. The D4
integration target was deliberately retained because its release executable
remains the raw artifact cited by the authoritative real-4K acceptance record.
The execution-time inventory/API/free-space values are operator-recorded; the
current workspace independently proves the deleted target's absence and the
protected source/evidence targets' presence.

This checkpoint closes D4 integration/build/launch, not the release. The exact
record is
`qa/SYNDOCAL_ALPHA11_MAIN_INTEGRATION_CHECKPOINT_2026-08-25.md`. Next action is
the peer `beta-v1.1.2` commit/push and live wired-LAN acceptance, with ASIO
alpha.12 integration continuing in parallel.

## 40. 2026-08-25 DJ-Link live-LAN preflight without acceptance promotion

The clean active KDMX checkout was
`codex/syndocal-v1.2` at
`1402a93069d8d630df66b1f682d0813b2d595faa`, equal to its upstream. The running
exact-checkout alpha.11 executable remained PID `114780`, size `57,888,768`
bytes, SHA-256
`522074E96A235310C9D39D3200E2A9D9B5C68429F0160E92318B568A0FE5AEE2`. A
read-only network observation found no TCP `9100` listener. The built product
does contain the dedicated `/dj-link` ingress; it was not listening because
Web Remote had not been explicitly started and DJ Link defaults to disabled
with no selected Show-LAN bind IP. No listener or process was changed by this
observation.

**Historical 2026-08-25 topology observation — do not execute as current setup:**
FOH Ethernet 3 / interface index `28` was then active at `192.168.1.34/24`, making
`ws://192.168.1.34:9100/dj-link` the endpoint observed at that checkpoint. Its
Windows profile was `Public`, and no exact Syndocal/KDMX or TCP `9100` firewall
rule was present. The WSL virtual address `172.30.208.1` was also active and can
be ambiguous in an address-only picker. At that historical checkpoint, binding
would have had to select the Ethernet 3 address explicitly; this is retained as
past evidence only and is not a current endpoint instruction. LAN reachability
and any narrowly scoped firewall change remain recorded physical acceptance work,
not an inference from source.

The DJ PC supplied a useful Stage 1/process preflight but not a completed gate.
Its `rb-output` state reported version 1.1.2 development/unverified, Agent
enabled, `CustomMIDI1` port 1 ready, and Pedal F13/F14/F15 listening. Hook data
for Deck 2, BPM, playback, Loop, and mixer was observed. F13/F14 actions were
recorded and local MIDI was sent, but Syndocal dispatch failed because the
endpoint was disabled and not listening. The new setup API returned 404 from
the running peer, proving that process was an older build and must be replaced
by an identity-bound current 1.1.2 artifact before the two-process run.

Two implementation boundaries remain in addition to the physical matrix. The
P1 restart gap is that listener enabled/bind/start state is not restored and
the process-local token is regenerated, so a configured peer cannot perform
restart/next-show reuse without manual repair. Close it with secure
machine-local secret storage outside project/renderer artifacts, NIC identity
rehydration with stale failure, an explicit armed/autostart policy, and restart
reconnect tests. The P2 NIC ambiguity is that address-only discovery can offer
virtual/tunnel interfaces such as WSL; expose adapter identity and reject or
warn on those candidates.

**SUPERSEDED / DO NOT EXECUTE — historical preflight evidence only:** this
2026-08-25 observation previously named `syndocal-envelope-v1` for the then-current
1.1.2 peer. It is retained solely to preserve the observed older-peer boundary and
must not be used as a setup, selection, or handshake instruction.

**Current executable next action — exclusive envelope:** build and launch the
identity-bound current peer, verify its setup API, explicitly select the currently
provisioned show LAN (FOH Syndocal `Ethernet 4 / 192.168.50.1`; DJ-agent local NIC
`192.168.50.2`) and `syndocal-envelope-v3`, rotate/copy the show-once token, start
the Syndocal Remote listener, and first prove authenticated wired HELLO/ACK. The
current setup and handshake must explicitly reject flat, v1, v2, and retired
Master-only envelope selections, HELLO capability sets, event names, or payload
fields; none may fall back, normalize, or
proceed. Only then may the twelve-row hardware sequence proceed. This observation
checks none of HW-4.1 through HW-4.12: the DJ/Pedal submatrix remains 0/12, and the
whole-product accepted denominator remains exactly 19/71 (26.8%).

## 41. 2026-08-25 authoritative near-show completion objective

The immediate completion denominator is now the physical next-show scenario in
`qa/SYNDOCAL_NEAR_SHOW_READINESS_2026-08-25.md`. It requires the exact
`DSF2026.dvc` import boundary; representative Lighting; temporary deterministic
LED `1920x1080` and projector `3840x2160` video with a `1920x1080` editor;
authored `人生オーバー` and `惑う星` Timelines; wired authenticated any-deck DJ-Link
trigger, position and loop synchronization; pedal-mediated DJ-to-band
continuation; BPM-aware automatic transition to `惑う星`; real ASIO; three
physical displays; persistence/restart; native artifact identity; and closure of
every discovered fail-closed, clean-break, browser/native, warning, and operator
defect.

This is a stricter scenario gate, not a reduction of the 71-row product ledger.
No browser, loopback, short device smoke, partial Timeline, or generic native
launch may promote this objective. The final artistic Lighting design and final
video assets remain user-authored after the complete product path is proven with
representative scenes and deterministic temporary media.

## 42. SUPERSEDED / HISTORICAL — 2026-08-27 show-critical DJ machine authority checkpoint

Commit `dcf6e524eddfaf79a54856af458efef202c079e1` is the pushed source checkpoint
for HW-4.11. The old process-local-token/address-only/listener-manual-repair path
is replaced by V2 non-secret machine settings, separate Credential Manager
primary/rollback records, monotonic generation high-water, a crash-compensated
journal, durable disarm cleanup, exact NLM `(network GUID, adapter GUID, IPv4)`
trust, and armed DJ-only startup through the single shared Remote listener.
Retired, malformed, future, stale, ambiguous, missing, or partially committed
authority fails closed. Authenticated HELLO bearer bytes are zeroized before
retained session/canonical state, and status/Debug/persisted JSON do not expose
the token.

The fixed-source gates passed with exact MSVC 14.44 and zero first-party
warnings: Syndocal `119/119`, I/O `38/38`, protocol `13/13`, control-plane
`1/1` over `482` routes, frontend build, invoke `422`, command routing
`130/31/30/411`, localization `3560/3560`, five-size Remote scroll, and Timeline
context-menu/performance proof. Independent Terra xHigh review approved without
a blocking P0/P1/P2 finding. The authored `New Scene` label is now protected in
all discovered Cue Pad/Armed/report views while operation text remains
localizable.

This was source completion, not show acceptance. The running untouched artifact
is still alpha.15 PID `46120` with an established DJ-PC socket; alpha.16 native
build/launch, real Windows credential/NLM restart, rb-output `1.1.6` deployment,
and HW-4.1 through HW-4.12 are pending. The DJ matrix remains **0/12**. Cleanup
was inventory-only and reclaimed `0` bytes because the reviewed incremental
tree remains hardlinked to external `.tmp.driveupload` paths. The next safe
show-critical action is the exact alpha.16 native build and full hardware matrix
after the operator authorizes replacement of the live alpha.15 process.

## 43. SUPERSEDED / HISTORICAL — 2026-08-27 bounded DJ machine test extraction

Pushed commit `684ecc01ceba141047f49cc595337f138719b749` reduces the
show-critical `dj_link_machine.rs` compilation unit from `2,644` to `1,491`
lines by moving its unchanged private child test module to
`src/tests/dj_link_machine_tests.rs`. All `38` tests and `125` assertion-macro
occurrences remain; independent Terra review found no semantic or visibility
change. Exact MSVC 14.44 Windows proof passed `37/37`, with the one remaining
test unchanged behind its non-Windows cfg, and first-party warnings remained
zero. This is a refactor-only checkpoint: the live alpha.15 process, cleanup
state, HW-4 **0/12** matrix, and all alpha.16 native/hardware boundaries in
section 42 are unchanged.

## 44. SUPERSEDED / HISTORICAL — 2026-08-27 transaction-capacity and zero-owned-FX source checkpoint

Pushed commit `33912001bfaa4c7b7d1ad10fec6bf0313e2debc2` closes the source
capacity leak from dead-owner terminal project receipts. Compaction requires the
exact owner triple, a closed zero-inflight Committed/Cancelled receipt, and its
exact lane; active, missing, Pending, and indeterminate state rejects before an
owner transition. The old terminal-residue path is not retained as a fallback.
Exact MSVC 14.44 focused proof passed `5/5`, broader `project_transaction_`
proof passed `10/10`, the frontend transaction checker passed, and first-party
warnings were `0`. Independent Terra xHigh review found no P0/P1/P2.

Pushed commit `627e35b32008c4087bd344f6531ccd6a5707d13d` clean-breaks the
last-target FX policy. The Tauri production helper keeps Cue existence and
target validation, then uses the published engine command for an empty
replacement; effect-only Scenes now persist a valid zero-FX body, while unknown
or malformed targets remain fail-closed. The frontend always offers Save Recall,
shows the empty-list result beside the action row, and removes the retired
helper, warning, and CSS path. Exact-linker focused Syndocal/engine tests passed
with zero first-party warnings; recall checker, localization `3560/3560`,
TypeScript/Vite build, formatting, and diff checks passed. Independent backend
and frontend Terra xHigh reviews ended with no P0/P1/P2.

The same current-source audit corrected three stale ledger descriptions without
rerunning or promoting a native gate. Commit `aed77a2` already uses a typed,
strict Bank rename request; preserves non-null missing Bank selection as an
unavailable identity with dependent actions locked; and rejects dangling or
cross-Bank active Cue references before queueing, normalization, or runtime
activation. The focused source/unit gates are present. Fresh native
Bank/Timeline, polling-removal/delete-successor, malformed-project, and
no-effect-activation proof remains open.

Both commits are source completion only. Native Bank/Scene authoring,
save/restart, zero-FX UI/runtime/publication-failure acceptance, and alpha.16
artifact proof remain open. The operator's alpha.15 PID `46120` was not stopped
or restarted and still owns the established DJ-PC connection. The reviewed
HW-4 companion is `qa/DJ_HW4_OPERATOR_RUNBOOK_2026-08-27.md`; it checks no
hardware row, so HW-4 remains **0/12**.

## 45. SUPERSEDED / HISTORICAL — 2026-08-27 bounded media/audio playback test extraction

Pushed commit `6cfb71c4e23141103e5ce33542b404d759828e95` moves the inline
`media_audio_playback_tests` child module from `app/src-tauri/src/main.rs` to
`app/src-tauri/src/tests/media_audio_playback_tests.rs`. The outer
`#[cfg(test)]`, private child-module relationship, `use super::*`, all 34 helper
and test function names/order, 31 tests, and 166 assertion-macro invocations are
preserved. No production symbol, runtime path, feature, or visibility changed.
`main.rs` shrank from 130,174 to 128,582 lines; the new focused test file is
1,578 lines.

Supervisor proof initialized `vcvars64.bat -vcvars_ver=14.44`, pinned the exact
Community linker through
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`,
and verified that exact path first in `where.exe link.exe` before Cargo. The
exact command was `cargo test -p syndocal --no-default-features --locked
media_audio_playback_tests`; it passed `31/31` with 1,099 tests filtered and
zero first-party warnings. Rust formatting and diff checks passed. Independent
Terra xHigh comparison approved with no P0/P1/P2 and a matching normalized body
hash. This is test-only maintainability work, not a native or show acceptance
claim; PID `46120`, HW-4 **0/12**, and every alpha.16/hardware boundary remain
unchanged.

## 46. SUPERSEDED / HISTORICAL — 2026-08-27 DJ peer v1.1.7 source authority

Peer commit `2577496767cf4ca8c8abdcadddbb891c7a609a32` is pushed and
upstream-equal on `beta-v1.1.2` at version `1.1.7`. It replaces the retired
v1.1.6 direct-Stop profile with one exact Stage 1 `filter-then-stop` route: HPF
CC16 `64 -> 127 / 1000 ms / 50 ms`, one planned Cue/Stop Note37, one independent
correlated `DJ_RELEASE`, and best-effort reset to CC16 `64`, with no fade MIDI.
Measured F14 loop authority extends through `1/64`; prediction is restricted to
true non-response. Stage 2 remains Timeline-only and emits zero MIDI. MASTER is
diagnostic; any exact mapped actually-playing deck may be admitted, and a MASTER
change cannot replace the owner.

The final peer suite passed `404/406`, with `0` failures and two explicit
REAL-package opt-in skips; syntax was `30/30`, focused proof `17/17`, and
independent review found no P0/P1/P2. Distribution artifacts are not yet built,
and source proof checks no HW-4 row. The next DJ-PC action is to pull the exact
commit, create and validate `C:\SyndocalShow\dj-agent-v1.1.7.json`, import
`CustomMIDI1-Syndocal-v1.1.7.csv` into the now licensed Rekordbox MIDI surface,
then execute the current operator runbook. HW-4 remains **0/12** until direct
hardware and bounded fault evidence is recorded.

## 47. SUPERSEDED / HISTORICAL — 2026-08-27 alpha.16 standard native artifact checkpoint

The exact-checkout historical alpha.15 PID `46120` was re-resolved and solely
terminated immediately before the release build. The first exact-linker attempt
failed closed before product linking because `FFMPEG_DIR`/`LIBCLANG_PATH` were
absent. The gate was restarted from the beginning with the complete installed
shared FFmpeg SDK and LLVM path, while retaining `vcvars64.bat
-vcvars_ver=14.44`, the absolute Community `14.44.35207` Cargo linker pin, and
that linker first in `where.exe link.exe`.

`pnpm --dir app tauri build --no-bundle` passed from source
`15b2d3f8886d11d571b2fc804ca9b774afff4746` in `2m 53s`, with zero observed
first-party warnings. The resulting `1.2.0-alpha.16` standard executable is
`59,024,384` bytes with SHA-256
`CB2CCA8102F71C187A35F645EAEBB6C5A99052A48F2575B59782D44D57274C4C`.
It is active as sole exact-path PID `74772`, responsive with title `Syndocal`.
Computer Use confirmed the sole owned window, native restore state, and
`1920x1032` maximized client. The I/O right pane rendered Web Remote and expanded
DJ Link in the same contained native view.

This closes the standard alpha.16 build/launch/maximize gate only. DJ Link was
not armed and HW-4 remains **0/12**. Show-ASIO physical/fault/endurance/latency,
three displays, representative show content, and full rehearsal remain open.

## 48. SUPERSEDED / HISTORICAL — 2026-08-27 alpha.17 native and DJ peer v1.1.8 handoff

The alpha.17 KDMX source checkpoint is pushed and upstream-equal at
`fb5d18fdf898a1435bed173ddd17934a04a97897`, product version
`1.2.0-alpha.17`. The alpha.17 native artifact is
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`, size
`59,021,824` bytes, SHA-256
`8B35A0F89ED6FA9A1BF8B1929BFA323F7F6250DF059D6314CCE7DDD6D39EBE45`,
Product/FileVersion `1.2.0-alpha.17`, with exactly one responsive maximized
Syndocal window at PID `57640`. Focused engine `dj_link_` proof passed `25/25`,
Syndocal `dj_link_dispatch_` passed `8/8`, the three-display harness passed
`80/80`, the frontend build passed, detached `check:release` passed including
`169` ASIO packaging assertions, and first-party warnings are `0`.

The current DJ-PC source target is branch `beta-v1.1.2`, package version
`1.1.8`, clean and upstream-equal at exact commit
`0f3e8c6851857c8542c132a89a7d44289002b1f5`, exact adapter
`syndocal-envelope-v3`, and the checkout-external `dj-agent-v1.1.8.json`
configuration with its versioned `CustomMIDI1` mapping. Production remains on
version `1.1.8`. The latest non-Master Deck 2 router-to-real-MIDI seven-byte
proof passed focused `12/12`; the full peer suite passed `415` total / `413` pass
/ `0` fail / `2` intentional skips. The commit is pushed, clean, and independently
reviewed GO. v1.1.7 is historical only and no peer or hardware row is promoted
from it.

The accepted Stage 1 F13 contract is: start HPF and synchronously route exactly
one correlated `DJ_RELEASE` at the accepted edge, before local MIDI completion;
Syndocal turns the DJ loop OFF and relinquishes DJ clock authority only when the
Timeline is already playing, without seek, jump, start, or changes to position,
playing state, child transport, or Follow. Independently, the DJ Agent completes
HPF CC16 `64 -> 127`, ChannelFader CC17 `127 -> 0`, Cue/Stop Note37, then resets
HPF/fader to `64`/`127`; both ramps are `1000 ms` with `50 ms` updates, and a
local MIDI failure never suppresses Release delivery. The existing Stage 2
`timeline-control` F13/F15 `-4/+4` beat-jump and F14 absolute-loop boundary is
unchanged and still requires direct confirmation; it is not this Stage 1 path.

This is software/native evidence, not physical acceptance. DJ deployment,
authenticated two-process behavior, Rekordbox/MIDI/pedal evidence, and HW-4
remain **0/12**; the 2026-08-29 completion deadline and 2026-08-30 performance
remain in force.

## 49. SUPERSEDED / HISTORICAL — 2026-08-27 alpha.18 source/native/UI authority

Section 52 supersedes this checkpoint for current execution. At this historical
checkpoint, KDMX source was `1.2.0-alpha.18` at branch
`codex/syndocal-v1.2`, exact `HEAD`/upstream
`db4eefc348b01ee05dd2dc87945afa85de8803e`. The alpha.17 source, native
artifact, process identity, and hash recorded in section 48 remain historical
and are not re-bound to alpha.18. The required `pnpm --dir app tauri build
--no-bundle` passed with exact MSVC 14.44 linker-first setup and zero first-party
warnings. The resulting alpha.18 artifact is:

- path: `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`
- Product/FileVersion: `1.2.0-alpha.18`
- size: `58,740,224` bytes
- SHA-256: `841068E08F80EB877FBA919FB86D3F52B3D4444314B47ABEF995BFA593E8D4F9`
- LastWriteTimeUtc: `2026-08-27T05:38:40.1840641Z`
- launch: exactly one responsive process, PID `80264`, title `Syndocal`, window handle `854080`, `IsMaximized=true`, start `2026-08-27T14:38:57.8350806+09:00`

The existing alpha.17 native artifact returned
`wired_candidate_discovery_failed` during wired refresh; its typed live
diagnostic exposed the old `Structural DuplicateIpv4Address` code. The root
cause was a typed `sin_addr` read against the `SOCKADDR_IN` `+8` padding. The
source fix corrects that layout handling and drops COM objects before
`CoUninitialize`. Exact MSVC/FFmpeg live-unit and hardware-enumeration checks
passed and identify Ethernet4 `192.168.50.1` as eligible. The native screenshot
confirmed Web Remote/Security/Endpoints/DJ Link/Standby in the same disclosure
stack and showed two wired candidates. The attempted native wired-binding
refresh click remains unconfirmed because foreground PID retrieval failed. The
five-viewport setup harness independently confirmed listener empty→count `0`→
Ethernet4 `192.168.50.1`→count `1`, one invoke per phase, and disabled mutation
controls; this does not promote the native click to accepted.

The Web Remote source layout now uses the same connection disclosure stack as
DJ Link and Endpoints without shrinking controls; native visual confirmation is
verified above, while the direct refresh-button click remains unconfirmed. The
standard and dedicated Setup I/O browser contracts pass all
five viewports, including `1280x720`. Independent review found and closed an
adjacent fail-closed defect where rejected DMX network-route buttons mutated the
protocol draft before reporting `no state changed`; the candidate-only path now
leaves the full draft and all `128` route signatures unchanged and invokes no
retired output command. Remote Start/Stop is asserted exactly once and in order;
independent re-review is GO with P0/P1/P2 all zero. Focused root revalidation of the MASTER clean break passed
protocol `7/7`, runtime `5/5`, I/O `37/37`, frontend/build, live, and static
checks under the required single-thread standard gate; independent review is GO.
A parallel I/O race is baseline-existing and is not acceptance evidence;
the exact MSVC 14.44 / locked full-workspace rerun passed `1164` pass / `0` fail
/ `11` intentional hardware-media ignores in the Syndocal binary target with
zero first-party warnings. The touched full-gate fixture repairs were
independently reviewed GO with P0/P1/P2 all zero; frontend invokes are `422`,
localization is `3559`, and the frontend production, viewport/setup, and warning
ratchets are green. The clean release gate `4fc443d` passed after staging seven
pinned DLLs. Main-checkout `check:release` remains intentionally blocked by the
tracked, unchanged runtime-inventory hard-link alias to
`C:\Users\kouty\Documents\.tmp.driveupload\867492`; no alias deletion or file
replacement was performed. Native build/launch/layout integration is verified;
the direct refresh-button action remains unconfirmed.

`app/dist` has already been freshly rebuilt and its old stale marker is `0`.
The ignored peer `dist` remains stale at `277,382,202` bytes but is outside the
production/source checkpoint. Cleanup of the prior alpha.17 temporary tree
(`394,438,512` bytes) remains policy-blocked with no bytes reclaimed. The
reviewer baseline
`C:\Users\kouty\AppData\Local\Temp\kdmx-head-baseline-review-20260827-1246`
is now `49,009,359` bytes after `885.6 MiB` was reclaimed by `cargo clean`;
direct cleanup remains policy-blocked. HW-4 remains **0/12**; this continuation
does not claim native or physical acceptance.

## 50. 2026-08-27 production title-selector editor checkpoint

This source-only checkpoint starts from branch `codex/syndocal-v1.2`, exact
`HEAD`/upstream `129d08d76141e7e69363a454d429bc3acca6b50e`. The DJ Link
mapping editor now authors the production `titleContains = 人生オーバー`
selector with an explicit Deck 1 fallback, while `Use Current Track` remains an
exact Content ID or exact Title + Artist operation. The retired editor path
silently stripped `titleContains` and `fallbackDeck`; the new path preserves,
normalizes, validates, edits, displays, and persists both fields.

The selector policy is isolated from the 29-thousand-line `App.tsx` in the new
191-line `app/src/djTrackMappingPolicy.ts`. It applies trim plus NFC without
case folding, makes selector forms mutually exclusive, canonicalizes unused
fields to null, and rejects more than 128 mappings, duplicate trimmed IDs,
duplicate normalized selectors, invalid Timeline IDs, control characters, and
values over 256 UTF-8 bytes. Panel validation occurs before the parent callback,
so a rejected add/edit keeps its draft and renders the error locally; App still
performs the final independent normalization before accepting state.

`pnpm --dir app run check:dj-link`, `check:localization`, TypeScript, and the
frontend production build passed. Localization is **3564/3564** with **0**
unprotected user-data labels; the App chunk is **498.92 kB** (gzip
**152.36 kB**) without a Vite warning. The dedicated Setup I/O gate passed all
five viewports, including `1280x720`, without resizing typography, controls, or
targets. `git diff --check` passed with only Git LF-to-CRLF notices and the
modified frontend emitted **0 first-party warnings**. Independent Terra xHigh
review is **GO** after it found and verified the draft-retention P1 fix. Ox was
unavailable, so this is the recorded narrow review exception.

No native build, deployed process, saved show project, or physical DJ/pedal
acceptance is claimed. The running alpha.18 process remains the prior artifact.
The next safe action is to finish the separate Follow-hold/F13 source tranches,
advance the prerelease, and execute the exact native and HW-4 gates.

## 51. 2026-08-27 alpha.19 Follow-hold / +4-only source checkpoint

KDMX `1.2.0-alpha.19` is committed and pushed at
`41faefc054a3c37cef81cfd2e69b4e3f3df5ab4f`.
`hold_first_destination_measure=true` clean-
breaks arbitrary non-Cut authored Follow duration: settlement is exactly one
source admission meter-aware bar, then success installs an indefinite runtime-
only loop from destination `0` through its first meter-aware measure. Authored
loops and project persistence remain unchanged. Both the `人生オーバー` C-melody and
post-Follow destination hold release only via F13 loop-off; F14 is the absolute
toggle and F15 is the only beat jump, `+4`. `-4` and all Stage 2 MIDI are rejected.
`transitionHoldActive` is required in authoritative Timeline state as a
post-Follow diagnostic, not as the F13 gate. F13 requires exact running
timeline/play-session/pedal-owner/Release correlation plus authoritative
`loopActive:true`, so the same absolute loop-off releases either the ordinary
C-melody A-B loop or the post-Follow hold. Completed Follow rebase is required
only for the latter destination authority.

Completed Follow rebases DJ runtime authority only for the exact released play
session, Release receipt/pedal owner, and completed source/target pair. Abort,
fault, stale generation, target/session mismatch, or receipt mismatch never
rebases. Exact mapping is any-deck; the zero-positive fallback is fresh Deck 1,
and multiple positives prefer Deck 1 then the lowest valid deck. The strict wire
preserves actual `artist` with `title` whenever `contentId` is absent.

MSVC 14.44 focused proof includes Syndocal DJ Link `119` pass / `0` fail / `1`
ignored. The exact serial locked-workspace gate is `2622` pass / `0` fail / `15`
intentional hardware-media ignores with first-party warnings `0`; frontend
build, 422 invokes, localization `3568/3568`, Mapping/Stage/Follow focused gates,
fmt, and diff all pass. Independent Terra xHigh re-review is GO with P0/P1/P2
zero. The clean non-OneDrive release gate passed at exact `41faefc` after
frozen-lock dependency restoration and staging seven pinned FFmpeg DLLs. Peer
v1.1.9 is committed/pushed at `b03d66a87b8d9dcdedfbd9b5c395bda7df7e0eec`
with `453` pass / `0` fail / `2` skips. PID `80264` remains the old alpha.18
artifact; alpha.19 native/HW acceptance is pending and HW-4 stays **0/12**.

## 52. 2026-08-27 alpha.20 Stage and Setup I/O operator-layout native checkpoint

This checkpoint began on `codex/syndocal-v1.2` at
`7cda8c8fbf04bde6efc705ef59a65dd5816da551`. Product metadata is
`1.2.0-alpha.20`; exact commit
`03b70cd14a285a41c63cfd1d9b3bd89c025eec16` is the authoritative
Stage/Setup-I/O source checkpoint. The implementation diff is frozen after
independent review. This is not a deployed or physical-hardware acceptance
claim.

The old Stage mapping surface mixed independent X/Z conversion with a
square-only interaction frame while the rectangular viewport, glyph scale, and
nested minor/major grid used different rules. At zoom this made segmented
bars/strobes appear to change relative size, shifted visual grid phases, and
could clamp a drag to the old square. The new contract uses one physical world
scale for fixture position, footprint/segment pitch, grid, snap, and pointer
conversion; its viewport is rectangular and keeps both grid layers on the same
origin. The old absolute selections drawer overlaid the Stage and hid fixtures.
It is replaced by normal workspace flow, retaining selection access without
obscuring the mapped area. Fixture transforms now require an authoritative
post-write confirmation; batch movement is serialized and a rejected or
partially rejected batch reports failure without a false success message.

The old Stage limits editor used a tall raw-value form that separated the
movement map from its inputs. The new compact, local two-column Dimmer /
Movement presentation keeps exact raw values and accessibility while adding
human-readable percentage/degrees and a compact square movement map. It does
not globally shrink controls or typography.

The old Setup I/O split connection settings into fixed quadrants, leaving empty
space below Control Mapping while an open DMX connection had too little room.
The new operator surface uses five connection cards (DMX, MIDI, OSC, Web
Remote, DJ Link) and one full-width workbench with its own body scroll. DMX
Input and Merge live with DMX output; advanced Web security/endpoints stay
separate from DJ Link authority and token controls. Inactive Web/DJ workbenches
are not mounted, so neither path duplicates listener, authority, or token
handling.

Focused proof is complete. `check:mapping-stage-geometry`,
`check:mapping-live-segments`, `check:bar-beams`,
`check:control-stage-persistence-failure`, `check:control-stage-fixture-edit`,
`check:mapping-live-snapshot`, and `check:stage-labels` passed.
`check:mapping-viewport-conformance` passed `5/5`; the strongpoint proof kept
the physical cell pitch at `18.1818` for both `4x1` and `4x10`; the Control
fixture edit browser gate proved position move/Undo, yaw `30 -> 90`, a single
`22 x 22` yaw target, and zero yaw handles after clear-pick. Workspace operator
passed `28`; project storage passed; frontend invokes are `422` exact;
localization is `3582/3582` with zero unprotected strings. The Tauri wrapper
checker passed `231` assertions including `27` hostile cases. Workspace split,
Setup I/O, and remote disclosure scroll each passed all five viewport cases.
The production build completed without a Vite warning; the final App chunk is
`499.67 kB`. The `frontend-typescript-vite-windows` warning ratchet is
baseline/current `0/0`, with zero first-party and zero third-party warnings.
`git diff --check` passed with only Git line-ending notices.

After the source commit, the broad `check:dj-link` gate exposed a QA-only
contract drift: its static assertion still required Web Remote `<details>` to
be the direct child of `ioDisclosureStack`, while alpha.20 intentionally wraps
Web and DJ disclosures in mutually exclusive Solid `<Show>` boundaries so the
inactive authority surface is not mounted. The checker now requires both exact
lazy wrappers (`surface !== "dj"` for Web and `surface !== "web"` for DJ), rather
than accepting the retired direct-child layout. The live browser helper also
requires Web selection to mount Web/Security/Endpoints/Standby with no DJ Link,
and DJ selection to mount DJ Link with no Web disclosures; restored Web must
again have no DJ Link. `node --check`, `pnpm --dir app run check:dj-link`, and
all five `check:viewport -- --setup-io-only` cases pass; the DJ gate reports
both mapping policy and frontend contract passed, and every viewport reports
`remoteScroll=1`. This QA/docs follow-up does not change the alpha.20
product/native source commit or close any HW-4 row.

The first native attempt failed before product linking because `FFMPEG_DIR` was
not set; no artifact from that attempt is accepted. The full gate restarted
from exact-path process inspection, `vcvars64.bat -vcvars_ver=14.44`, the pinned
Community linker
`C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`,
and exact `where.exe link.exe` first. With the pinned FFmpeg 8.1.2 shared root
and `C:\Program Files\LLVM\bin`,
`pnpm --dir app tauri build --no-bundle` completed in `2m11s` with zero
first-party warning lines. The exact artifact is
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`, `58,777,088`
bytes, Product/FileVersion `1.2.0-alpha.20`, SHA-256
`E8100D160158034A63901EA1BF775EC06A48EFF4D97CAF454D0378C0D6988D7D`,
LastWriteTimeUtc `2026-08-27T14:02:29.3834470Z`. Launch observed exactly one
responsive process, PID `18456`, title `Syndocal`, handle `19597984`. Computer
Use confirmed a maximized `1920x1080` window by the native
`元のサイズに戻す` title-bar action. Native visual QA confirmed the five-card
connection deck, one full-width DMX workbench with internal scroll, rectangular
Stage, and non-overlay context pane. Switching Web/DJ cards through native UI
automation was not accepted because the accessibility element cache rejected
the action; the five-viewport browser gate covers that behavior but is not
promoted to native proof.

Independent Terra xHigh final reviews are **GO** with P0/P1/P2 zero after the
Control yaw browser assertion was strengthened. Ox callable capacity was
unavailable, so the narrow recorded exception applies. The product-version
surface is synchronized and the Show-ASIO metadata self-test passed `38`, but
`pnpm --dir app run check:release` remains fail-closed before metadata
validation because `qa/FFMPEG_WINDOWS_RUNTIME_INVENTORY.json` has the external
hard-link alias `C:\Users\kouty\Documents\.tmp.driveupload\867492`. Neither
link was deleted or replaced. This blocks a public release/installer claim, not
the accepted local source/native checkpoint.

Checkpoint cleanup inventory was read-only. Logical file-length totals are
repo `278,371,465,649` bytes (`259.25 GiB`), `.git` `217,408,416` bytes,
`target` `277,042,280,989` bytes (`258.02 GiB`), `app/node_modules`
`545,338,492` bytes, and `app/dist` `5,108,607` bytes. `target/debug` accounts
for `242.23 GiB`, including `115.02 GiB` incremental and `112.06 GiB` deps.
No deletion was performed. After `03b70cd` was clean, pushed, and
upstream-equal, read-only command
`& .\qa\harnesses\invoke-syndocal-build-cache-cleanup.ps1` returned `Mode=Plan`,
`Outcome=Blocked`, `Blocker=WriterOwnershipTopologyUnverifiable`: Adobe Creative
Cloud Libraries `node.exe` PID `61616` referred to a missing positive parent PID
`49864`, so ownership could not be proved. The exact allowlist remained only
`target/debug/incremental`; it contained `72,840` files / `123,505,799,091`
logical bytes (`115.02 GiB`), was only `0.122` days old versus the seven-day
minimum, and separately reproduced `HardlinkDetected` with link count `5`.
`tools/asio-bridge/target` remains outside the reviewed allowlist, and the legacy
harness remains plan-only. Preserve release, QA, ASIO, Show-ASIO, and debug
dependency artifacts until writer topology, age, hard-link, and exact-path
review gates all pass.

Real fixture placement against the supplied Daslight project, DMX output and
input/merge, MIDI, Rekordbox, DJ Link, pedal HW-4 (`0/12`), target-PC deployment,
three-display operation, and the remaining ASIO matrix are still unverified.
Those are the next show-critical boundaries.

## 53. 2026-08-27 current Deck 1 fallback acceptance boundary

The current controlled peer remains rb-output `1.1.9` at exact
`b03d66a87b8d9dcdedfbd9b5c395bda7df7e0eec`. Its production
`titleContains = 人生オーバー` selector deliberately owns the operator-requested
zero/multiple-positive policy: zero positives may select only fresh playing
Deck 1 after `deck1MetadataWaitMs = 1400`; multiple positives prefer fresh
playing Deck 1 and otherwise the lowest valid positive Deck. Master state never
participates. KDMX's explicit authored `fallbackDeck = 1` accepts that peer
decision and never grants fallback authority to Deck 2.

This policy is not the retired implicit content-ID-to-title fallthrough. Exact
content and exact title + artist selectors do not cross-fall through, and
duplicate/ambiguous authored fallback mappings remain fail-closed. The earlier
HW-4.2 wording incorrectly classified the explicitly configured Deck 1 case as
a negative; the current acceptance split keeps negative identity cases outside
that fallback in HW-4.2 and requires the zero/multiple-positive demonstration
in HW-4.3. No runtime source changed in this adjudication, and HW-4 remains
`0/12` pending real wired Rekordbox/DJ/pedal evidence.

Current-head focused revalidation used exact MSVC 14.44 and its pinned linker
first: engine DJ-Link `28/28`; Syndocal DJ-Link `119` passed / `0` failed / `1`
intentional live-network ignore; first-party warnings `0`.

## 54. 2026-08-28 historical alpha.21 Setup I/O operator checkpoint

Product/FileVersion at this historical checkpoint was `1.2.0-alpha.21` on
branch `codex/syndocal-v1.2`. The I/O/native source checkpoint is the committed and
pushed `536742db968b242164349c34dd6940fe3ced8e92`
(`feat(setup): streamline the I/O operator workbench`). The separate token-free
show structural preflight is committed and pushed at exact
`b35d3ba351b29caaedfb7d0c6f4e29fc84580e83`. Tool/helper `node --check` and
`node qa/tests/show-structural-preflight.mjs` passed; the valid fixture exits
`0`, while legacy, missing, usage, and unknown inputs exit `2`. Independent
Terra xHigh final review is GO with P0/P1/P2 all `0`.

Setup I/O presents five selectors (DMX, MIDI, OSC, Web Remote, DJ Link) and
one full-width workbench whose body is the single scroll owner. Its selector row
uses `role=tablist`/`role=tab`, current-selection semantics, and an exactly
labelled `tabpanel`; the active workbench keeps only a compact useful label/state
instead of redundant selected-workbench summary text. Service and quick actions
remain separate sibling controls with independent hit targets and do not change
inactive selection. The dedicated browser contracts prove real pointer
hit-testing (including the
fifth DJ tab), real keyboard selection (Enter/Space, ArrowLeft/Right,
ArrowUp/Down, Home/End), exact tab/tabpanel relation, strict overflow
(`scrollHeight > clientHeight`), and no competing scrollports among DMX/MIDI/
OSC/Web/DJ descendants. The
remote disclosure contract opens Web disclosures, scrolls the bottom
`remote-standby` summary/control into view, and requires visibility plus a
center hit-test. Web/DJ five-way and bottom-scroll evidence is browser-contract
evidence only and is not native visual proof.

Focused Setup I/O and remote-disclosure-scroll contracts passed at all five
supported viewports (`1920x1080`, `1920x1032`, `2048x1152`, `1366x768`,
`1280x720`). `check:localization` passed `3577/3577 (100%)` with `0`
unprotected labels. Frontend `tsc --noEmit` plus Vite build passed (`294`
modules) without build warnings; JS syntax checks and `git diff --check`
passed. Independent Terra xHigh final review is GO with P0/P1/P2 all `0`.

The native gate initialized `vcvars64.bat -vcvars_ver=14.44`, pinned
`C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`
in `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`, and verified that exact path
first in `where.exe link.exe`. `pnpm --dir app tauri build --no-bundle`
produced `target/release/syndocal.exe`, Product/FileVersion
`1.2.0-alpha.21`, `58,778,112` bytes, SHA-256
`F73F1AD8F9E8229B8ACB713AE0C10C36E199C878D869A2E98B554190FC908FA3`.
Exactly one responsive maximized exact-checkout Syndocal window was observed
(PID `41912`). Native visual proof is limited to DMX/MIDI/OSC selector and
workbench paths; Web/DJ five-way and bottom-scroll proof remains browser-only.

The token-free show structural preflight is read-only, consumes no real token,
launches no show process, changes no runtime/project/hardware state, and cannot
advance HW-4. Its strict boundary excludes production `.sdc` and media,
target-DJ-PC checkout/config/token/NIC, LAN/HELLO/ACK, Rekordbox, pedal/MIDI,
reconnect/restart, three-display, and physical-output proof.

HW-4 remains exactly `0/12`; real production `.sdc`/media, target-DJ-PC, LAN,
and reconnect evidence remain unverified. Cleanup remains Plan-blocked: the
clean upstream-equal `1d8fc4f` rerun reported `Mode=Plan`, `Outcome=Blocked`,
`Blocker=WriterOwnershipTopologyUnverifiable` because writer PID `61616` has
missing positive parent PID `49864`, `Candidates=[]`, `PlannedLogicalBytes=0`,
and `ReclaimedLogicalBytes=0`, so no deletion was performed. The exact 12-path
hardlink remediation was content-preserving with no content diff.

## 55. 2026-08-28 historical alpha.22 authoritative Scene-create source checkpoint

Historical exact source authority is committed and pushed at
`cc201ad40927f0631236680392025f81e51c6130` with synchronized
`1.2.0-alpha.22` metadata. Empty and Capture-current Scene creation now share
the strict tagged backend operation `syndocal.scenes.create.v1` through
`create_scene_authoritative_v1`. The retired backend `create_empty_cue`, the
renderer-ticketed `create_cue_from_current`, and fixture-local Scene synthesis
are removed from callable manifests and inventory.

The unified publication path uses the existing admitted snapshot protocol.
Definitive pre-admission failure restores A and may release only the tail ID;
admitted ACK loss is indeterminate, retains the Scene ID, and fences every
subsequent project mutation. The terminal receipt carries the exact committed
`cue_id`; the frontend refresh verifies the same ID in the requested Bank.
Stale or malformed authority, unknown fields, receipt mismatch, Bank mismatch,
and retired route use all fail closed without a compatibility branch.

Exact MSVC 14.44 supervisor gates passed engine `3/3`, app Scene `3/3`, Empty
Scene `2/2`, and exact control-plane route/handler/inventory `3/3`, with zero
first-party warnings. Focused frontend routing/project transaction, TypeScript/
Vite (`294` modules), release metadata, fmt, and diff gates passed. Independent
Terra xHigh review was GO with P0/P1 zero; its ignored-dist P2 was closed by
regeneration and a zero-match retired-route scan.

This checkpoint deliberately has no alpha.22 native claim. The current
responsive/maximized exact-checkout artifact remains alpha.21 from source
`536742d`, PID `41912`, `58,778,112` bytes, SHA-256
`F73F1AD8F9E8229B8ACB713AE0C10C36E199C878D869A2E98B554190FC908FA3`.
The next native build is deferred until the missing canonical composition-to-
video-output assignment path is closed under a new prerelease ordinal, avoiding
an unnecessary second show-time process replacement. Physical outputs,
projector mapping, authored `.sdc`, and HW-4 remain open gates.

## 56. 2026-08-28 historical alpha.23 output-routing / Setup I/O / frontend source checkpoint

The historical source authority is exact committed and pushed, upstream-equal
`5e7d27df7f5864449d4838782f6eca2f9b81d360` on branch
`codex/syndocal-v1.2`, with synchronized product metadata
`1.2.0-alpha.23`. This was a source-only checkpoint: no alpha.23 native
artifact has been built, launched, or used for native UI acceptance.

Old path: composition-to-output assignment could enter through a raw direct
routing mutation. New path: the canonical
`assign_video_output_composition_v2` operation is the only R4 composition-to-
output route. The raw/retired `set_video_output_routing` route is absent from
the protocol, control-plane registration, frontend manifest, and App mutation
path. A successful assignment requires a terminal authoritative receipt,
fresh snapshot/render-plan refresh, and exact output-to-composition
verification. A definitive pre-admission failure restores the prior state;
admitted ACK loss is indeterminate, retains ownership/fence state, and uses the
existing restart boundary. Reason: keep composition/output authority and
success classification on one fail-closed path without a local snapshot or
compatibility route.

Setup I/O now presents five selectors (DMX, MIDI, OSC, Web Remote, DJ Link)
inside one single-scroll workbench. Web Remote and DJ Link are direct selected
workbenches; DMX input is explicitly optional. Advanced remote safety limits
and connection/client information remain available behind disclosures. The
wide operator layout uses the requested two-column reflow while preserving
typography, controls, and hit targets.

Rarely used App-only phase-1 actions are loaded through a dynamic
`phase1Actions` import. This creates a separate lazy chunk while preserving
labels, error semantics, and call timing; no UI behavior or output authority
is changed by the split.

Supervisor gates initialized the exact MSVC 14.44 Community toolchain, pinned
the absolute `Hostx64\\x64\\link.exe`, and verified it first in
`where.exe link.exe`, ahead of Git. Protocol tests passed `1/1`, engine tests
`1/1`, App tests passed `3/3`, and first-party warnings were `0`. Output
control, ownership, routing, and frontend checks passed `128/31/30/412`.
Setup I/O viewport and remote-scroll contracts passed `5/5` viewports each;
localization passed `3578/3578` with no unprotected labels. The final
frontend build transformed `296` modules and emitted `App-CNx-MFOg.js` at
`499.68 kB` and the `phase1Actions` chunk at `2.84 kB`, with zero Vite chunk
warnings. `check:release` and `check:release:self-test`, format, and
`git diff --check` passed. Independent Terra xHigh source review was GO.

At this historical alpha.23 checkpoint the running native process was the
alpha.21 artifact: PID `41912`, `58,778,112` bytes, SHA-256
`F73F1AD8F9E8229B8ACB713AE0C10C36E199C878D869A2E98B554190FC908FA3`.
Physical DJ/MIDI/pedal acceptance, HW-4, real output, target-DJ-PC, LAN,
reconnect, and three-display evidence remain unverified; this checkpoint does
not claim final completion. Historical alpha.24 native evidence is recorded in
section 57.

## 57. 2026-08-28 historical alpha.24 Timeline Follow rearm source/native checkpoint

At this historical checkpoint, product metadata was `1.2.0-alpha.24`. Its source authority was
exact committed, pushed, and upstream-equal
`5eb1888906613851c4c49e66717a270f13bbd7b5`. Alpha.23 remains historical at
exact pushed commit `5e7d27df7f5864449d4838782f6eca2f9b81d360` and is not
relabelled as alpha.24. The release path performs the actual DjLink clock
handoff only, arms an enabled, unfenced, eligible Follow, performs no
seek/play/jump or authored-state mutation, and admits exactly one Follow
transition at the next natural boundary. For an authored non-Cut first-measure
hold, settlement uses one source measure and installs the destination's first
meter-aware measure as a runtime-only hold; the 5/4 proof does not mutate
authored state. The alpha.24 native build was complete; its evidence is recorded
below.

The exact supervisor gate passed `29/29` engine `dj_link` with warnings `0`.
The MSVC 14.44 linker
`C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`
was pinned and verified first in `where.exe link.exe`, ahead of Git.
`check:release` and `check:release:self-test` passed. Independent Terra xHigh
source review was GO. The required `pnpm --dir app tauri build --no-bundle`
passed in `2m35s` with Vite/Rust first-party warnings `0`. Product/FileVersion
is `1.2.0-alpha.24`; `target/release/syndocal.exe` is `59,803,136` bytes,
SHA-256
`8FF5CF6670F592DFFE53477E8E14BF95CD3FCF81916FC23CAD58E7354763874D`, and
LastWrite `2026-08-28T05:07:33.7560788+09:00`. Exactly one responsive process
and window was verified: PID `63016`, title `Syndocal`, responsive `true`,
handle `66658934`. Computer Use verified the unique process/window and the
accessibility system button `元のサイズに戻す`, proving maximized state. The
frontend build transformed `296` modules and emitted `App-CNx-MFOg.js` at
`499.68 kB` plus lazy `phase1Actions` at `2.84 kB`.

The StandardRelease three-display acceptance runner is pinned to the exact
alpha.24 Product/FileVersion, `59,803,136`-byte size, SHA-256, and source commit
above; Show-ASIO remains a separately licensed, independent authority. The
runner's deterministic self-test passed `88/88` with warnings `0`, covering
clean equal-HEAD and valid clean descendant-HEAD operation while retaining the
artifact source identity. Physical Apply was not run because the loaded project
is `Untitled.sdc` and no real output IDs/labels are configured.

The alpha.21 artifact remains historical: Product/FileVersion at that
checkpoint was `1.2.0-alpha.21`, PID `41912`, `58,778,112` bytes, SHA-256
`F73F1AD8F9E8229B8ACB713AE0C10C36E199C878D869A2E98B554190FC908FA3`.
Physical DJ/MIDI/pedal acceptance, HW-4, and real-output acceptance remain
unverified. This is not a final-completion claim; the remaining physical/show
gates are open.

## 58. 2026-08-28 CURRENT post-alpha.25 cleanup Plan

After alpha.25, branch `codex/syndocal-v1.2` was clean and upstream-equal at
exact `HEAD`/upstream `aaccd77e445e027544bbf57e46280bc10bd30cdf`. The read-only
command
`& .\qa\harnesses\invoke-syndocal-build-cache-cleanup.ps1` was attempted twice;
both Plan attempts returned `Mode=Plan`, `Outcome=Blocked`, and
`Blocker=WriterOwnershipTopologyUnverifiable`, with the exact message:
`Writer ancestry is missing positive parent PID 49864 for writer PID 61616.`

A read-only observation afterward showed PID `61616` is Adobe Creative Cloud
Libraries `node.exe` under
`C:\Program Files\Common Files\Adobe\Creative Cloud Libraries\libs\node.exe`,
with current `ParentProcessId` `61456`. The prior ancestry could not be
re-proven, so cleanup remains fail-closed. `Candidates=[]`,
`PlannedLogicalBytes=0`, and `ReclaimedLogicalBytes=0`; Apply was not run and
no deletion occurred. The older `HardlinkDetected` cleanup result remains
historical and is intentionally unchanged.

## 59. 2026-08-29 historical alpha.28 source/integration checkpoint

At this historical alpha.28 checkpoint, the source authority was branch
`codex/syndocal-v1.2` at
`8153ebb37a9517aad91c0da6ad06de9a80db2a1a`, equal to its upstream at this
checkpoint. Product metadata is synchronized at `1.2.0-alpha.28`. The active
alpha.28 implementation changes remain uncommitted; this documentation
checkpoint makes no commit or push claim. Earlier sections retain their
recorded evidence and are not relabelled; this section is historical alpha.28
status and is superseded by the then-current alpha.30 section below.

Verified source evidence recorded at that historical checkpoint:

- `pnpm --dir app run build` passed and the frontend build transformed `302`
  modules.
- `pnpm --dir app run check:release` passed, including release metadata, ASIO
  packaging `169` assertions, ASIO v3 contract `22` assertions, Timeline
  PROGRAM/CUE bus `11` assertions, and the existing video route/window gates.
- Prior focused ASIO preflight evidence passed `17/17`, `11/11`, and `15/15`,
  with first-party warnings `0`. This is preflight/source evidence only.

Production code integration remained pending despite the isolated source and
preflight results. The alpha.28 native release build, native launch/window
acceptance, ASIO loader smoke, and all physical/hardware rows (including MOTU
M4, M32/DL16, serial DMX, DJ/MIDI/pedal, real ACK, and reconnect) remain
unverified. No historical native or hardware checkbox is changed or promoted
by this section.

The first safe resume action at that historical checkpoint was to complete and
independently review the alpha.28 production integration, rerun the exact gates
from a stable tree,
then perform the required native and hardware acceptance before any commit or
push.

## 60. 2026-08-29 historical alpha.29 lifecycle checkpoint and hybrid-output requirement

At the historical alpha.29 checkpoint, branch `codex/syndocal-v1.2` had
advanced to committed and pushed source checkpoint
`54a4ffcce0e2029d9f0aecc713ae4436228a4d3c`; product metadata was synchronized
at `1.2.0-alpha.29`. That checkpoint made Ready cancellation and explicit
Normal return authoritative, serialized ASIO lifecycle mutations, and removed
the show-ASIO warning inventory without warning suppression. Independent review
was GO with P0/P1 zero.

Verified evidence is show-ASIO `cargo check` warnings 0, application tests
1409 discovered / 1397 passed / 0 failed / 12 ignored, frontend contract 59
static plus 28 runtime assertions, TypeScript/Vite 302 modules, and complete
`check:release` including packaging 169, ABI v3 22, and Timeline bus 11.
`pnpm --dir app tauri build --no-bundle` succeeded with the exact MSVC 14.44
Community linker first and warnings 0. Native window acceptance remains open:
the exact responsive process completed Tauri setup but exposed only its 16x16
single-instance window, not a user-facing Syndocal window. No native-window GO
is inferred from the successful build.

At that historical checkpoint, the show requirement was a deliberate expansion
of the previous
single-stream acceptance: logical PROGRAM and CUE must be assignable to
separate operator-selected physical output devices. The first physical target
is TOPPING E2x2 ASIO 1/2 for PROGRAM plus an explicit headphone/WDM endpoint
for CUE. Same-ASIO remains the shared-clock mode. Hybrid mode must expose its
dual-clock limitation, route every CUE source to exactly one selected endpoint,
keep the ASIO CUE lanes silent, forbid automatic fallback, and fence CUE on
PROGRAM stop/fault or endpoint loss. The hybrid implementation and its
physical acceptance were pending at alpha.29; that wording is historical and
is not the alpha.30 status recorded in section 61. The next safe action at that checkpoint was
the separately owned hybrid profile/runtime/UI tranche followed by independent
review and exact native gates.

## 61. 2026-08-29 historical alpha.30 hybrid PROGRAM/CUE source checkpoint

At that checkpoint, product metadata was `1.2.0-alpha.30` on branch
`codex/syndocal-v1.2`. The alpha.30 source/UI tranche was present in the dirty
worktree and had no accepted alpha.30 source commit. It added arbitrary
PROGRAM/CUE device assignment in two explicit modes: `CueDelivery::SameAsio`
keeps PROGRAM and CUE on one selected ASIO stream and shared clock, while
`CueDelivery::ExplicitWdm` keeps PROGRAM on ASIO and sends CUE to the explicitly
named WDM endpoint whose enumerated topology is part of the selection. Timeline
CUE clips, generated Click/Guide, and the explicit CUE test path use the same
logical CUE route. Missing, ambiguous, stale, changed, or failed route/session
state remains visible and fail-closed; no ASIO/PROGRAM/default-device fallback
or CUE leakage to PROGRAM is accepted by the source contract. Session and
generation fences cover activation, publication, timeline preparation, and
retirement paths.

Focused source checks are provisional. The previously recorded full Rust result
`1426 discovered / 1414 passed / 0 failed / 12 ignored` predates later source
fixes and is stale; it is excluded from final alpha.30 evidence. Any focused
result from the dirty tree must be rerun after the later fixes from a clean,
pushed source checkpoint. No alpha.30 native build/launch/window, real-device
audition, physical output routing, or show completion has been verified.

The oversized audio-runtime extraction from `app/src-tauri/src/main.rs` is
deferred until after show acceptance. Moving ASIO/CUE ownership and lifecycle
boundaries before the show is a pre-show risk, so this checkpoint records no
module split or extraction completion. This section is historical and is
superseded by the then-current alpha.31 checkpoint below; the alpha.28/29
“pending/not implemented” statements above remain historical.

## 62. 2026-08-29 HISTORICAL alpha.31 Timeline authoring-monitor native checkpoint (partial)

The product metadata at that checkpoint was `1.2.0-alpha.31` on branch
`codex/syndocal-v1.2`. The alpha.31 Timeline-authoring source and the normal-build
cfg correction are pushed through
`602b96a8fcb0de3fd3a3e281324550fe1d7b5630`.

The Normal route now has a dedicated Timeline authoring monitor. Existing
`FollowProgram` behavior remains available. With operator-selected
`ExplicitDevice`, every Timeline media-library clip, whether its logical bus is
PROGRAM or CUE, and generated Guide/Click material use one exact selected WDM
endpoint. Endpoint selection is arbitrary and machine-local; exact name,
topology fingerprint, attachment revision, and settings revision are checked
before publication and preparation. Missing, ambiguous, stale, changed, or
failed endpoint/session state is visible and silent; there is no OS-default,
PROGRAM, or Show-ASIO fallback.

Show ASIO remains a separate owner and clock-domain path. Entering it retires
all Normal-authoring Timeline sinks and in-flight preparation, including when
no Normal PROGRAM stream is open. Returning to authoring requires an explicit
Normal route and output selection. This is an owner/generation boundary, not a
second implicit playback path.

Verified source evidence for this checkpoint is:

- The exact MSVC 14.44 Community linker was pinned and resolved first in
  `where.exe`; first-party warnings were `0` in the recorded gates.
- ASIO-enabled full Syndocal suite: `1456 discovered / 1444 passed / 0 failed /
  12 ignored`.
- Focused ASIO media-audio suite: `72/72`.
- TypeScript, Vite, release metadata/checking, audio-output control/panel,
  Timeline-audio, command-routing, and localization checkers passed.

The normal alpha.31 native no-bundle build passed from clean pushed HEAD
`602b96a` with exact MSVC 14.44. Its `60,756,480`-byte executable has SHA-256
`6F9BF17A2802A2FC5F8935E8EFFEB0C57CA4A3A21B245C68BFA62315152C7F4A`, and
exactly one launched process was responsive and the exact native window was
maximized through its verified process handle. Product-path enumeration found
one selectable `Music (Elgato Virtual Audio)` endpoint; its exact name and
topology fingerprint were persisted while the app was stopped, then the app
was relaunched successfully. Audible confirmation remains unverified. Physical WDM/ASIO output,
serial DMX, DJ Link, reconnect, venue routing, and show completion remain
external gates. Ox was unavailable for this tranche; under the documented
narrow exception, an independent Terra xHigh source review returned GO with no
P0/P1.

The oversized audio-runtime extraction from `app/src-tauri/src/main.rs` stays
deferred until after show acceptance because moving ownership/lifecycle
boundaries before native and venue acceptance is a pre-show risk. The first
safe resume action is the real endpoint audition; installer/updater and
dedicated show-ASIO builds remain separate.

## 63. 2026-08-29 HISTORICAL alpha.32 source/native integration checkpoint (hardware pending)

Section 62 is preserved as historical alpha.31 native evidence. At this
historical checkpoint, product metadata advanced to `1.2.0-alpha.32` on
`codex/syndocal-v1.2`; no alpha.31 artifact identity is relabelled. The source
integration includes two pushed, reviewed pre-version checkpoints; the alpha.32
product identity itself begins with the subsequent synchronized version
checkpoint:

- `fedf6c48fbab59b3f0c643da402fcc026ee74fbe` makes Child Timeline PROGRAM/CUE
  audio use one canonical 250..=4000 millirate for source position, Rodio
  speed, drift, and inverse Sink seek. Nested fractional rates round once;
  invalid or ambiguous conform rates refuse audio attachment. A negative
  source offset stays unattached until its exact root-output boundary through
  nested, loop, seek, and restart paths. Lighting keeps its existing position
  contract. Independent Terra xHigh rereview is GO with no P0/P1/P2.
- `53d70baacbc2c9ed3f719eaf2e67aab1766cf487` makes Windows release-candidate
  metadata crypto preflight complete before any executable inspection, passes
  inspectors and archive tools only verified-byte materializations, and keeps
  cleanup/source swaps visible and fail-closed. The implementation review is
  GO; actual RC acceptance remains NO-GO until real signed NSIS/MSI/updater and
  Windows runtime identity evidence exist.

Recorded gates are Timeline audio `31/31`, ASIO media `80/80`, protocol
nonserialization `1/1`, and Windows candidate self-tests `125/43/4/140`, with
first-party warnings 0. `check:release`, format, and diff checks passed.

The synchronized product identity is pushed at
`5d40874c629f26d6e011252622b2886d39d8d40b`. The exact MSVC 14.44 normal
`pnpm --dir app tauri build --no-bundle` passed from that clean pushed HEAD in
`2m 52s` with first-party warnings 0. The resulting executable is `60,713,984`
bytes, reports Product/FileVersion `1.2.0-alpha.32`, and has SHA-256
`B04CE351A456C82715383A4430401F3DA4D824EC8813B32894ABE1BF98FBD90D`.
After inheriting the verified local FFmpeg runtime `bin`, exactly one
checkout-owned process launched as PID `89524`, remained responsive with title
`Syndocal`, and Computer Use verified the exact window was maximized.

Audible `Music (Elgato Virtual Audio)` authoring monitor, physical
ASIO/WDM/USB-DMX, DJ Link, reconnect, and show acceptance remain open. Camera/Screen Capture
4K/60 and device-supported 120fps are a separate alpha.33 tranche; current
capture remains 1280x720/30 until that reviewed implementation lands.

Cleanup was inventoried read-only for this checkpoint. `target`,
`app/node_modules`, `tools/asio-bridge/target`, and `app/dist` still exist; a
fresh exact-byte `target` traversal did not finish within the bounded 50-second
window and was stopped without filesystem changes, so no new exact byte count
is claimed. The latest completed exact inventory remains `428,244,808,551`
logical bytes for `target`. The reviewed recurring-cleanup harness is not
approved for Apply, so no deletion ran and reclaimed bytes remain 0.

## 64. 2026-08-29 CURRENT alpha.33 camera-capture source tranche (native pending)

The product identity is `1.2.0-alpha.33`; this is a source checkpoint and not
yet a versioned native acceptance. The old camera route
accepted a free-form DirectShow endpoint at fixed `1280x720` / `30 fps`. The
new source path uses an explicit current-generation DirectShow device/profile
catalog, persists only an opaque endpoint identity, and requires an exact
one-frame probe before Add. Its admission envelope is maximum `4096x2160`;
profiles above `1920x1080` are admitted at no more than `30 fps`, profiles above
`1280x720` at no more than `60 fps`, and capture rates up to `120 fps` only when
the device advertises that profile. Output presentation remains capped at
`60 Hz`; screen capture is unchanged at `1280x720` / `30 fps`. The acceptance
authority is [qa/CAMERA_INPUT_ACCEPTANCE.md](qa/CAMERA_INPUT_ACCEPTANCE.md).

On 2026-08-29, the connected `Insta360 Link` advertised `3840x2160` at
`30 fps`; `1920x1440`, `1920x1080`, and `1280x720` at `60.0002 fps`; and no
`120 fps` profile. The exact supervisor source gate passed capture `64 passed /
0 failed / 2 ignored` and control-plane `64 passed / 0 failed / 0 ignored`
under the exact MSVC `14.44.35207` linker pin, with first-party warnings `0`.

Independent review found P0 `0`. Source fixes close the bounded listing/memory
limit, complete child-process cleanup after post-spawn failures, and automatic
stale Active-row replacement after capture fault. Sustained-4K performance
remains open. A `4096x2160` RGBA frame is about
`33.75 MiB`; cloning it at `60 Hz` may approach `1.98 GiB/s` of copy traffic.
Direct FFmpeg preflight completed 150 RGBA frames at 4K30 and 300 at 1080p60,
both exit `0`; the Syndocal profile probe still proves one frame only, so
sustained 4K remains unverified and
no 4K60 or broad sustained-4K claim is made. Native alpha.33 build, UI, and
hardware probes remain pending.

Final independent source rereview is GO with P0/P1 `0`. Full app regression
passed `1196/0/7` without default features and `1232/0/12` with default
libav/Spout features. Frontend production build, localization `3631/3631`,
typed IPC inventory `440`, `check:release`, format, and diff gates pass with
first-party warnings `0`.
