# Syndocal completion flow — current execution authority

Date: 2026-08-19
Branch at creation: `codex/syndocal-v1.0`
Baseline before this document: `848d759846985cc3acf588356cfa3c996b4e2ef2`

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
`1.2.0-alpha.1`. The branch name may remain historical; artifact metadata and tags
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

- [ ] Advance and verify all product metadata as `1.2.0-alpha.1`.
- [ ] Replace stale resume instructions and percentages with current AI3 truth.
- [ ] Build Q1-Q4 coverage from every phase below; assign Supported/External/etc.
- [ ] Create W0 warning inventory and enforce W1 no-new-warning ratchet.
- [ ] Extend `check:release` with tag/previous-version/updater/artifact checks.

Exit: synchronized version metadata, warning baseline, current traceability/risk/
evidence ledgers, clean reviewed commit, and pushed handoff.

### Phase 1 — Generic project authority and atomic project mutations

- [ ] E1 generic Begin reply-loss, terminal recovery, live-owner transaction
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

- [ ] Extend the pure lease core into a bounded multi-lease registry with atomic
  Lighting/Video overlap handling.
- [ ] Add exact request receipts, same-ID/different-shape rejection, bounded lanes,
  rate limit, audit truth, and crash-safe/durable terminal recovery.
- [ ] Wire lease state into AppState, generation-stamped query, canonical commands,
  registry metadata, and current-process owner retirement.
- [ ] Revalidate exact lease owner/resource/generation/expiry at the final existing
  R4 Release/Arm/Take Over commit boundary while S0 remains independent.
- [ ] Orphan affected leases on project identity replacement without physical change.
- [ ] Canonicalize or fail-close MIDI, OSC, DMX mapping, Remote, shortcut, all/video/
  per-output release, and every other energizing legacy route.
- [ ] Fence New/Load/Recovery/Backup/Take Over through acknowledged physical
  retirement, project replacement, and separate explicit re-Arm.
- [ ] Prove stale owner, reply loss, restart non-reclamation, transfer races,
  10,000-call saturation, physical creation/teardown ACK, and hardware behavior.

Exit: all five AI3 roadmap categories accepted. Do not begin AI4 before this exit.

### Phase 3 — AI0-AI8 control plane completion

- [ ] AI0 complete source inventory/coverage gate; unclassified mutations fail.
- [ ] AI1 query/event schemas, snapshots, generations, gap/resnapshot, bounds.
- [ ] AI2 authored command bridge with E/R/H, owner incarnation, receipts, Undo.
- [ ] AI4 principals, pairing, grants, revocation, kill switch, Raw Input presence,
  one-shot consent, challenge/token expiry, and release-build bypass absence.
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
- [ ] L-TL5 Follow/crossfade, BPM slew, failure policy, and `Trans` Guide.
- [ ] L-TL7 Undo/Redo/save/reload group selection/focus, custom Guide TTS/device
  routing, native A/V/Lighting synchronization, fault, and viewport proof.
- [ ] M thumbnail/waveform/proxy/analysis identity, bounded background workers,
  cancellation, cache/eviction, predecode/prefetch/degraded operation, and cold/warm
  cache performance budgets.
- [ ] L authored Audio schema/migration/history and explicit ShowClock/audio/PTS
  master-clock, resampling/slew/seek/loop/underrun/device-fault policy.
- [ ] Extend the existing Web Remote listener with authenticated DJ Link events. Map
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
- [ ] Pass a second vendor's working driver.
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
- [ ] macOS and Linux real-machine display/media/audio/project save/reload; CI package
  launch alone is insufficient.
- [ ] Venue GPU maximum ISF/layer/Preview/Program/output/recording one-hour run with
  frame/drop/CPU/GPU/RAM/VRAM/operator-response logs.
- [ ] Two-machine real-switch crash/restart/partition/rejoin/device-loss rehearsal
  with zero simultaneous output and operator runbook.
- [ ] Pinned Daslight/SynapseRack/TouchDesigner version/license/hardware/content task
  comparisons, preserving first failures, unmeasured rows, timings, operations, and
  synchronized output evidence. Never infer parity from counts or loopback.

## 9. Distribution, legal, clean-machine, and publication

- [ ] Freeze supported Windows/macOS/Linux architecture/package/feature matrix and
  exact normal MIT/WASAPI versus separate ASIO artifacts.
- [ ] Windows Authenticode; Apple Developer ID, hardened runtime, notarization,
  staple/Gatekeeper; key custody/rotation/revocation and CI secret policy.
- [ ] Release BOM/SBOM and package/file-accurate licenses/notices for FFmpeg/libav,
  NDI, Spout/Syphon, ISF, GDTF/OFL, fonts/icons/samples/media, ASIO, Rust, and npm.
- [ ] Clean-machine download/hash/signature/install/first launch/open/save/import/
  output-disabled startup/file association; previous-version upgrade, interrupted
  rollback, repair, reboot, uninstall, and user-data retention on each platform.
- [ ] Signed updater N -> N+1, downgrade/wrong-channel/wrong-key/expired/corrupt/
  partial/offline/unavailable endpoint, rollback artifact, and emergency disable.
- [ ] Freeze version/tag/commit/release notes; publish SHA-256/manifests/artifacts;
  archive logs, locks, SBOM, notices, tests, native/hardware evidence under one ID.

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
7. migration/corruption/security/updater/clean-machine matrices;
8. signed/notarized artifact comparison and pinned commercial benchmarks;
9. Q5 query against exact tag, artifact hashes, logs, and evidence manifest.

Stop and report rather than claiming completion if any command selects zero tests,
any warning is unowned, any artifact differs from the evidence hash, any external
gate is unavailable, or any claim exceeds the proven platform/hardware boundary.

## 12. Exact resume point

After the documentation/version checkpoint containing this file:

1. preserve the bounded Windows W1 ratchet and finish the nine pending W0 warning
   configurations; do not claim W0, beta, or RC warning acceptance before the
   required matrix is complete and first-party warnings reach zero;
2. continue AI3 with the bounded multi-lease/resource-overlap registry and exact
   process-local request receipt/shape-conflict layer;
3. independently review it before AppState/R4 integration;
4. keep AI4, distributed ShowClock, ASIO completion claims, and public release closed.

## 13. 2026-08-21 bounded completion-flow update

The current AI3/DJ integration tranche has a frozen independent result of P0 0,
P1 0, and P2 0. The DJ transport reuses Web Remote, generation-fences the
irreversible handler boundary, recovers connection/admission state on handler
panic, distinguishes StateSync from triggers, and keeps all rekordbox/Pedal/MIDI
responsibility in `rekordbox-DJ-Link-ForPCDJ`. The topbar no longer carries the
Lighting/Video master sliders; no adjacent UI was reduced in size.

The focused Rust and frontend gates listed in the handoff pass, and the final
Windows native `--no-bundle` build completed in 2m12s with zero first-party and
zero Vite warnings. Seven locally measurable W0 configurations must now be
promoted from pending using exact artifacts/output markers and a reviewed
inventory-only checkpoint. macOS dev/release remain pending external evidence,
so this update does not set `requiredMatrixComplete=true` and does not claim beta,
RC, or release acceptance.

After the inventory-only checkpoint and its docs-only normal-mode proof, launch
the exact rebuilt executable, verify exactly one responsive maximized Syndocal,
then perform the requested five-display VJ output exercise and leave the final
operational state as editor plus LED panel plus projector. If physical
OutputControl consent rejects synthetic input, stop rather than bypassing the Raw
Input boundary and record the single human-keyboard action still required.

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

The exact Windows native executable is responsive and the sole main Syndocal window
is maximized. Five enabled Display outputs are configured against monitor indices
0..4. Physical fullscreen activation is still fenced because OutputControl R4
requires an acquired backend lease and the visible challenge code entered through
real Raw Input. Synthetic input must not be used to bypass that boundary. The next
hardware action is one coordinated physical-keyboard consent pass, followed by
opening all five output windows, verifying their screen origins, and leaving the
operator state as editor plus LED panel plus projector. Until that pass is recorded,
the five-display and final three-display acceptance boxes remain unchecked.
