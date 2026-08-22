# Syndocal completion flow — current execution authority

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
has 156 authored 4/4 measures at 170 BPM. Its fixed Guide chart announces Intro,
Verse, Pre Chorus, Chorus, Interlude, Breakdown, and Outro; each phrase begins on
the preceding measure's final beat, except the first Intro at frame zero. Measure
98 is performed exactly eight times and every pass announces `Looping`. `Bridge`
is deliberately suppressed so it cannot overlap the loop call, and measure 99
announces `Break` from its preceding beat. Measures 149-156 form one
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
audition outputs live outside Git at `C:\TEMP\syndocal-show-audio`: nine
48 kHz, PCM16, mono WAVs (per-song and connected click/Guide/mix stems) plus a
sample-frame manifest. Independent review recomputed all PCM samples and accepted
P0/P1/P2 as zero: 652 `人生オーバー` performance clicks, 840 `惑う星`
clicks, 1,492 total clicks, and 33 physical/semantic Guide events. Thirty-two
Guide onsets are exactly one beat before their target and the sole exception is
the frame-zero `人生オーバー` Intro. The connected boundary is frame
11,010,639; duration is 23,480,742 frames / 489.182125 seconds. Fresh export
reproduced every WAV SHA-256 exactly and no output clipped. These files prove the
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

This is the current dependency-ordered execution and handoff authority from the
present AI3 checkpoint to a truthful public release. It does not erase detailed
requirements. The following documents remain normative for their domains:

- `qa/CODEX_COMPLETION_ROADMAP_2026-08-13.md`: complete product requirements,
  traceability, severity, hardware, distribution, and final integrated gate;
- `qa/SYNDOCAL_AI_CONTROL_PLANE_ROADMAP.md`: AI0-AI8 architecture and safety;
- `qa/ASIO_INPUT_ACCEPTANCE.md`: mandatory Windows ASIO acceptance;
- `qa/REKORDBOX_STREAM_DECK_PEDAL_ACCEPTANCE.md`: mandatory wired DJ Agent,
  Master Track mapping, absolute Timeline Loop, Release, and peer/hardware acceptance;
- `qa/M4_IO_VALIDATION.md`, `qa/M5_RELIABILITY_VALIDATION.md`, and
  `qa/M6_RELEASE_VALIDATION.md`: physical I/O, soak, and release evidence;
- `RELEASE_STATUS.md`: historical release evidence and external blockers;
- `qa/CODEX_HANDOFF_2026-08-19.md`: exact current implementation checkpoint.

Older percentages, resume instructions, and v1.0/v1.1 completion statements are
historical when they conflict with this file. In particular, the 2026-08-13 75.5%
planning roll-up is not current release evidence, and its old Media A1 resume point
must not replace the current AI3 resume point.

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

1. Read `AGENTS.md`, this file, the domain authority, and the current handoff fully.
2. Run `git status --short`, `git log -3 --oneline`, compare local/remote HEAD, and
   preserve all unrelated or user-owned changes.
3. Write the requirement, non-goals, risk class, compatibility/migration boundary,
   file ownership, exact tests, native/hardware needs, and stop condition.
4. Delegate material implementation to Luna Max by default; use Terra High/xHigh
   for difficult implementation and Sol only when needed. Assign explicit files.
5. Assign a separate adversarial reviewer. The reviewer is read-only until the
   implementation owner reports a stable checkpoint.
6. While agents/builds run, advance non-overlapping investigation, test planning,
   documentation, or evidence work. Never let agents edit the same files.
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

The active development train advances from the long-lived `1.1.0` metadata to
`1.2.0-alpha.4`. The branch name may remain historical; artifact metadata and tags
must not derive a false version from the branch name.

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

Warnings are work, not harmless release log noise. The current Rust release build
contains many first-party unused/dead-code warnings, and Vite reports oversized
chunks. The following ratchet begins immediately:

Provisional observed baseline from the fresh `1.2.0-alpha.1` native build: the
Engine reported 9 warnings, the Syndocal release target reported 58 warnings, and
Vite emitted its oversized-chunk warning for multiple generated chunks. These counts
are log-level starting evidence, not the complete W0 feature/platform inventory and
must not be used as an allowlist. W0 must still recount every supported feature and
platform configuration and classify every first-party diagnostic.

1. **W0 inventory:** capture warning code, file/line, target/feature, owner, reason,
   and removal checkpoint for default release, tests, all targets, ASIO, NDI, Spout,
   macOS, and Linux. Separate third-party/build-tool warnings from first-party code.
2. **W1 no growth:** no checkpoint may add a first-party warning. A warning in a
   modified file is a tranche blocker. Handoff records baseline/current/delta.
3. **W2 removal:** use focused commits to remove unreachable compatibility code,
   narrow `cfg` boundaries, delete obsolete helpers, or add real production use.
   Do not hide debt with crate-wide `allow(dead_code)`, `-Awarnings`, fake reads, or
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

### 5.1 Current bounded W1 checkpoint

Implementation commit `3ee303f4ce7fed897e4d2473ddf80b4335b20591`
establishes the reviewed Windows warning ratchet for four configurations: default
all-targets 83/67 warnings/identities with 11/11 artifacts, release 83/67 with
11/11, tests `--no-run` 25/21 with 11/11, and isolated Spout 79/67 with 9/9.
The 52-group negative harness and independent review found no remaining P0/P1 in
that bounded gate. Native release verification reproduced Engine 9, Syndocal 58,
and one Vite chunk warning without growth.

The focused warning/P2 checkpoint
`a228ba5e492841618ce0262038c965b5519d1ddd` reduces the current first-party
occurrences without editing the baseline: default and release 83 -> 61, workspace
tests 25 -> 20, and isolated Spout 79 -> 57, with unchanged exact artifact coverage.
The current native release reports Engine 9 and Syndocal 41 warnings plus the same
Vite chunk warning. Its independent review returned P0 0, P1 0, and P2 0 for the
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

### Phase 0 — Rebaseline, version, warnings, and ledgers

- [x] Advance and verify all product metadata as `1.2.0-alpha.1`.
- [x] Replace stale resume instructions and percentages with current AI3 truth.
- [ ] Build Q1-Q4 coverage from every phase below; assign Supported/External/etc.
- [x] Create the structured W0 inventory and enforce the W1 no-new-warning ratchet.
- **Deferred outside the Windows target:** macOS dev/release warning enforcement.
  The global inventory may remain 11 enforced / 2 pending without blocking the
  current Windows completion target.
- [ ] Extend `check:release` with tag/previous-version/updater/artifact checks.

Exit: synchronized version metadata, warning baseline, current traceability/risk/
evidence ledgers, clean reviewed commit, and pushed handoff.

### Phase 1 — Generic project authority and atomic project mutations

- [x] E1 generic Begin reply-loss, terminal recovery, live-owner transaction
  liveness, renderer retirement, and stale delayed Commit/Cancel rejection.
- [ ] E2 authority bundle/generation consistency across every project mutation.
- [ ] E3 recovery durable handshake and crash/reply-loss behavior.
- [ ] E4 Save/Save As/template/backup reservation, journal, mapping flush, atomic
  replacement, and truthful terminal receipt.
- [ ] D1 machine/session cache read purity.
- [ ] D2 atomic PATCH/GDTF Repair with whole-batch prevalidation, Published ACK,
  allocator/cache/output rollback, and one history result.
- [ ] D3 server-side admission for every mutation, including raw Tauri paths.
- [ ] D4 Stage import/mutation identity fence, atomicity, Undo, and error truth.

Exit: no project/file mutation bypasses one recoverable authority/publication path.

### Phase 2 — Complete AI3 runtime/output bridge

- [x] Extend the pure lease core into a bounded multi-lease registry with atomic
  Lighting/Video overlap handling.
- [x] Add process-local exact request receipts, same-ID/different-shape rejection,
  bounded lanes, rate limit, audit truth, and restart non-reclamation.
- [ ] Add crash-safe/durable terminal recovery rather than reclaiming pre-crash
  authority or presenting process-local receipts as durable truth.
- [x] Wire lease state into AppState, generation-stamped query, canonical commands,
  registry metadata, and current-process owner retirement.
- [x] Revalidate exact lease owner/resource/generation/expiry at the final existing
  R4 Release/Arm/Take Over commit boundary while S0 remains independent.
- [x] Orphan affected leases on project identity replacement without physical change.
- [x] Canonicalize or fail-close code-side MIDI, OSC, DMX mapping, Remote, shortcut,
  all/video/per-output release, and every other discovered energizing legacy route.
- [ ] Verify those ingress policies through native clients and hardware rather than
  treating the generated source inventory as physical acceptance.
- [x] Fence project replacement through candidate/orphan receipts, generation checks,
  and an explicit re-Arm requirement.
- [ ] Prove New/Load/Recovery/Backup/Take Over retirement and re-Arm end to end with
  acknowledged physical output state.
- [x] Prove code-side stale owner/ABA, restart non-reclamation, transfer races,
  bounded 10,000-call saturation, and candidate/teardown ACK behavior.
- [ ] Prove durable reply-loss/crash recovery, native dangerous-action
  Yes/No/close behavior, physical creation/teardown ACK, and the five-display
  hardware behavior. The retired Raw Input challenge is not an acceptance item.

Exit: all five AI3 roadmap categories accepted. Do not begin AI4 before this exit.

### Phase 3 — AI0-AI8 control plane completion

- [ ] AI0 complete source inventory/coverage gate; unclassified mutations fail.
- [ ] AI1 query/event schemas, snapshots, generations, gap/resnapshot, bounds.
- [ ] AI2 authored command bridge with E/R/H, owner incarnation, receipts, Undo.
- [ ] AI4 principals, pairing, grants, revocation, kill switch, exact reviewed
  local/native consent policies, and release-build bypass absence. This does not
  restore the retired Raw Input/six-digit/Enter challenge.
- [ ] AI5 authenticated localhost sidecar, MCP, JSON-RPC/REST, WebSocket, discovery,
  bounded queues, process lifecycle, and no cached authority replay.
- [ ] AI6 administration UI for principals/grants/revocation/audit/health.
- [ ] AI7 adversarial parity/security/rate/reply-loss/fuzz/saturation proof across
  Tauri, MIDI/OSC/DMX, Remote, shortcuts, API, and MCP.
- [ ] AI8 release-native real external clients, clean install, hardware output,
  crash/restart, security review, and artifact inspection.

Exit: AI0-AI8 accepted without weakening local Blackout or claiming unattended R4/R5.

### Phase 4 — Output ownership, project swap, and ShowClock decision freeze

- [ ] F1 monotonic input generations for MIDI/OSC/DMX Learn/workers and stale callback
  retirement on mapping/project replacement.
- [ ] F2 full Lighting/Video/Both/Standby ownership across DMX, NDI, Spout/Syphon,
  Display, native windows, SDK resources, Take Over, teardown ACK, and explicit Arm.
- [ ] Freeze ShowClock transport/discovery/authentication/key rotation/replay,
  master-clock/slew/Hold, witness/fence/physical-interlock, mixed-version, and
  supported-network decisions.

Exit: local/project output ownership and replacement are accepted, and ShowClock
decisions are frozen. Distributed implementation remains blocked until Phase 5
Audio/recording/live-source clock and ownership semantics are complete.

### Phase 5 — Video, Timeline, media-derived data, audio, recording, live sources

- [ ] Reintegrate accepted Windows Clip Slot/Layer Bus/FX tranches into the full gate.
- [ ] C2 Clip Take and C4 mapping/Timeline integration.
- [ ] L-TL5 Follow/crossfade, BPM slew, failure policy, and `Trans`/`Complete` Guide.
- [ ] L-TL7 Undo/Redo/save/reload group selection/focus, fixed Guide device
  routing, native A/V/Lighting synchronization, fault, and viewport proof.
- [ ] M thumbnail/waveform/proxy/analysis identity, bounded background workers,
  cancellation, cache/eviction, predecode/prefetch/degraded operation, and cold/warm
  cache performance budgets.
- [ ] L authored Audio schema/migration/history and explicit ShowClock/audio/PTS
  master-clock, resampling/slew/seek/loop/underrun/device-fault policy.
- [x] Extend the existing Web Remote listener with authenticated DJ Link events. Map
  `DJ_MASTER_TRACK_ACTIVE` to project-owned Timeline starts, converge authored A-B
  Loop from absolute `DJ_LOOP_STATE`, and make `DJ_RELEASE` idempotently disable/resume.
  Do not add a second Agent/server or send rekordbox MIDI from Syndocal.
- [ ] Stable live camera/screen/NDI/Spout/Syphon/generator identity and availability,
  permission/fault/reconnect truth, and old-worker retirement.
- [ ] Recording `Idle -> Preparing -> Recording -> Finalizing -> Complete|Fault`,
  target reservation, disk/crash/encoder/timeout recovery, verified atomic artifact,
  two-PC ownership, and authoritative asset import.

Exit: maximum-condition one-hour A/V/Lighting/recording proof meets fixed budgets.

### Phase 6 — ShowClock, UI, Remote, security, migration, and supportability

- [ ] Implement ShowClock schema/simulator, authenticated peer sync and estimator,
  timestamped exactly-once actions, project/lease/audio/recording generation coupling,
  witness/fence, UI, and two-process then two-machine fault/soak proof.
- [ ] H1 reachability: every supported feature has a discoverable operator path and
  no dead/hidden route that only an internal command can reach.
- [ ] H2 shared shell: one truthful navigation/status/error/selection/focus contract
  across Setup, Edit, Control, Touch, native windows, and compact/full layouts.
- [ ] H3 Setup completion: Patch/GDTF/OFL, mapping, I/O, output/device configuration,
  validation, empty/error states, keyboard/pointer reachability, and native proof.
- [ ] H3 DJ Link setup: explicit Show-LAN NIC/bind address, dedicated token rotation,
  connection/session diagnostics, current Master Track, project Track-to-Timeline
  mapping CAS, disconnect recovery, and `Use Current Track`. No Pedal/MIDI controls.
- [ ] H4 Edit completion: Media, Timeline, Phase/Guide/loop/group/follow, FX, Stage,
  history/Undo/Redo, import/relink, save/reload focus, and native proof.
- [ ] H5 Control completion: live Lighting/Video/Audio, Cue/Clip/Take/Transition,
  Blackout/Arm/Take Over, recording, diagnostics, failure/recovery, and native proof.

- [ ] N Remote/Touch LAN/TLS exposure, pairing, Origin/Host, grants, rate/size bounds,
  RDM/TOD cancellation/ownership, parser/path/archive fuzz, dependency/SBOM/redaction.
- [ ] O supported `.sdc`/template/cache/protocol version matrix, golden migration
  corpus, hostile/corrupt input, fuzz, backup/recovery/upgrade compatibility.
- [ ] P generation-stamped status, redacted diagnostic bundle, updater wrong-channel/
  downgrade/signature/corruption/offline/rollback behavior, and operator/support
  startup/failure/takeover/recovery/shutdown runbooks.
- [ ] H native NVDA, High Contrast, color-independent states, 125/150/200% scaling,
  keyboard-only safety workflows, IME, dialog/popout focus, and reduced motion.

Exit: supported exposure, migration, recovery, accessibility, and diagnostics have
native evidence, not only browser/static proof. ShowClock partition/crash/rejoin
cannot create simultaneous output; automatic failover is not claimed without the
accepted strongly consistent witness/fence or physical interlock.

## 7. Mandatory Windows ASIO completion

ASIO is a Windows product-release requirement while remaining a separately licensed,
non-default artifact from MIT/WASAPI.

- [ ] Select GPLv3-separated artifact or Steinberg proprietary SDK agreement; package
  notices/source obligations and approve installer/updater separation.
- [x] Pass a second vendor's working driver (HOTONE 44.1 kHz / 2-channel / i32 /
  128-frame, 100 exact Start/Stop/Free cycles with no fallback).
- [ ] Pass advertised 44.1/48/96 kHz, 64/128/256 frames, format, mono/stereo/channel
  selection matrix without silent fallback.
- [ ] Pass occupied-driver, control-panel rate/buffer change, reset/resync, XRUN,
  unplug/replug, callback-gap, and no-callback fail-closed/recovery matrix.
- [ ] Pass matched one-hour ASIO and WASAPI soak with overrun 0, callback p99 below
  20% and max below 50% of buffer duration, capture-to-engine p95 <= 40 ms, and
  loss-to-zero <= 250 ms.
- [ ] Measure physical input-to-pixel latency and five matched TouchDesigner trials.
- [ ] Verify selection persistence, stale/ambiguous device lock, native UI telemetry,
  artifact feature identity, and absence of fallback in the final package.

Exit: every checkbox in `qa/ASIO_INPUT_ACCEPTANCE.md` is checked with exact artifact,
driver/device, raw logs, operator, date, and measurement source.

## 8. External hardware, comparative, platform, and venue acceptance

- [ ] Real Art-Net/sACN nodes and fixtures: addressing, RGB/wheel, Pan/Tilt, intensity,
  strobe, Blackout, reconnect, multi-universe, switch topology, and 44 Hz continuity.
- [ ] Enttec USB PRO/DMXKing long run; Open DMX logic-analyzer Break 176 us, MAB 16 us,
  frame period/failure; RDM/TOD discovery/correlation/timeout/cancel/removal.
- [ ] Physical MIDI Note/CC/Clock/MTC/feedback/All Notes Off; OSC and TouchOSC/iPad/
  Android Remote over wired/Wi-Fi with p50/p95/p99/max latency.
- [ ] Physical DJ-PC Pedal -> `rekordbox-DJ-Link-ForPCDJ` local MIDI plus wired
  Agent -> Syndocal acceptance: preload/non-Master non-trigger, Master Track start,
  absolute repeated Loop divisions, Filter isolation, idempotent Release, local
  operation during disconnect, State Sync, ACK, and next-use readiness, with exact
  device/software/repository/NIC/mapping/timestamp evidence.
- [ ] Dual display/HDMI/fullscreen/DPI/refresh/unplug/reorder/GPU reset; NDI/Spout/
  Syphon, camera, and screen-capture fault and one-hour matrices.
- **Deferred outside the Windows target:** macOS/Linux real-machine display,
  media, audio, project save/reload, and package acceptance.
- [ ] Venue GPU maximum ISF/layer/Preview/Program/output/recording one-hour run with
  frame/drop/CPU/GPU/RAM/VRAM/operator-response logs.
- [ ] Two-machine real-switch crash/restart/partition/rejoin/device-loss rehearsal
  with zero simultaneous output and operator runbook.
- [ ] Pinned Daslight/SynapseRack/TouchDesigner version/license/hardware/content task
  comparisons, preserving first failures, unmeasured rows, timings, operations, and
  synchronized output evidence. Never infer parity from counts or loopback.

## 9. Distribution, legal, clean-machine, and publication — deferred

The six former completion checkboxes in this section are outside the active
Windows-local target: platform/package matrix, Authenticode/Apple signing,
BOM/SBOM and notices, clean-machine install/upgrade/uninstall, signed updater,
and public tag/artifact/evidence publication. Reopen them only when distribution
becomes a product goal; they are not counted in the active 71-item denominator.

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

## 12. Exact resume point

Resume from the current Windows-first checkpoint in this order:

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

The focused Rust and frontend gates listed in the handoff pass, and the Windows
native `--no-bundle` build, completed with zero first-party and zero Vite warnings.
The seven locally measurable rows were subsequently promoted; the inventory is now
11 enforced / 2 pending. macOS dev/release remain pending external evidence, so
`requiredMatrixComplete` stays false and this does not claim beta, RC, or release
acceptance.

The exact rebuilt executable was launched as one responsive maximized Syndocal and
five Display targets were enumerated. The Raw Input failure recorded at that time
is historical because the feature was later removed. Do not repeat a five-display
exercise for the active target; the final pass retains exactly the editor + LED
panel + projector state.

## 14. 2026-08-21 warning-P2 promotion result

The seven locally executable warning rows are now enforced with zero first-party
warnings: Windows ASIO loader, NDI, separately licensed ASIO bridge, frontend,
native release, and Linux dev/release. The promotion is reproducible through the
inventory-only `--promote-zero-warning` gate and host-specific execution; ordinary
baseline immutability was not relaxed. Exact coverage is recorded in
`qa/warnings/warning-inventory.json`. The required matrix is 11 enforced / 2
pending. Both remaining rows are macOS-only and therefore keep
`requiredMatrixComplete=false`; this is a bounded P2 resolution, not beta/RC or
whole-product completion.

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
