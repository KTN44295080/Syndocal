# Syndocal 2026-08-30 show completion handoff

Status date: 2026-08-29 JST

This is the concise authoritative resume note for the final show-critical tranche. It supersedes chat-only status, but it does not supersede the detailed acceptance documents named below.

## Source authority

- KDMX checkout: `C:\Users\kouty\Documents\KDMX`
- Branch: `codex/syndocal-v1.2`
- Current alpha.28 source parent HEAD and upstream: `8153ebb37a9517aad91c0da6ad06de9a80db2a1a`. The alpha.28 working-tree changes remain uncommitted at this documentation checkpoint; no commit or push is claimed here.
- DJ Agent checkout: `C:\Users\kouty\Desktop\rb-output`
- Branch: `beta-v1.1.2`
- DJ Agent committed HEAD and upstream: `a13d7bff59db5e7c00e19655f87c69db7cb52005` on `beta-v1.1.2`; its worktree was clean at the recorded checkpoint.
- The DJ Agent operator-return path received independent source-review `GO` with no P0/P1/P2. The external full regression passed `506 tests / 504 passed / 0 failed / 2 skipped` with first-party warnings 0. DJ-PC pull/restart, strict preflight, active runtime version, real ACK, and physical pedal acceptance remain external gates.
- KDMX product metadata is synchronized at `1.2.0-alpha.28`. Source integration and the exact normal `--no-bundle` native executable/window gate are recorded below. Normal installer/updater inspection, the dedicated show-ASIO artifact, and hardware acceptance remain open.

### 2026-08-29 alpha.28 integrated source checkpoint

- The previously open application integration is now present on the working
  tree. PROGRAM/CUE render, output runtime, Timeline output/transport, output
  router, normal-output boundary, bridge-v3, and preflight logic are split into
  dedicated modules instead of adding those implementations to the already
  oversized `main.rs`. The only final `main.rs` repair at this checkpoint is a
  deterministic test-startup fence; production behavior was not loosened.
- The final independent ASIO drain review is `GO` with no P0/P1. Stop fences
  the callback context before join, Fault uses one state-lock interval, stale
  queued Test/Solo blocks are silenced through the two-callback drain, and
  arbitrary invalid-width/partial/non-finite blocks fail to full-width silence.
  Supervisor evidence is `165 passed / 0 failed` for exact `asio_` focused
  tests with first-party warnings 0.
- The full parallel Syndocal show-ASIO suite passed from a fresh rerun after
  repairing two test-only races: `1366` discovered, `1359 passed`, `0 failed`,
  `7 ignored`, first-party warnings 0. The repairs use a test-only router slot
  rather than competing for the production process owner, and wait for the
  engine startup snapshot before atomic no-mutation comparisons. Focused proof
  after repair was `25/25` router, `48/48` Timeline CUE, `3/3` ASIO Timeline
  Play, and `1/1` for each DJ rejection boundary.
- The final accepted default workspace gate ran with the CI-equivalent
  `--test-threads=1` setting and passed `2708` tests with `0` failures and `17`
  explicit hardware/GPU/long-duration ignores. First-party warnings were 0.
  A deliberately broader parallel `--all-features` diagnostic is not counted
  as acceptance: it first lacked the NDI runtime DLL path and later exposed
  unrelated parallel native/socket-process instability. The exact `io` suite
  passed serially (`166 passed / 0 failed / 2 ignored`), and the accepted full
  workspace rerun is the serial result above.
- Release metadata and frontend contracts passed: ASIO packaging `169`, ABI v3
  `22`, Timeline PROGRAM/CUE UI `11`, project-open bootstrap, video output
  routing/window observation, and TypeScript/Vite production build with `302`
  transformed modules. `cargo fmt --all -- --check`, three Show-ASIO script
  syntax checks, and `git diff --check` pass; first-party warnings remain 0.
- The local-only artifact authority is now split into artifact source commit
  `S`, evidence HEAD `E`, and source branch `B`. The checker rejects dirty or
  unpushed evidence, a wrong branch, non-ancestor `S`, merges, D/R/T changes,
  non-allowlisted evidence paths, and source mutation followed by revert. Its
  schema is a clean-break v3 with manifest `commit=S` and `sourceBranch=B`.
  The final source identity contains `70` exact paths, including
  `app/src/uiLocalization.ts`; every Git authority query rejects repository,
  index, object, and config overrides and disables replacement objects.
  Supervisor self-tests pass: checker `155`, runtime staging `16`, build
  orchestrator `43`, and three-display harness `90/90`, with native/Cargo/
  process actions explicitly not run by those self-tests. Independent final
  rereview is `GO` with no P0/P1. PATH-resolved Git executable pinning remains
  non-blocking P2 hardening.
- Exact normal native acceptance passed after removing two release-only
  dead-code warnings with correct cfg boundaries: `pnpm --dir app tauri build
  --no-bundle`, first-party warnings 0. The exact executable is 60,599,296
  bytes, SHA-256
  `3080A3664B4657D6B6D6EFE76448DBE13912CA118262E78C0AE02B8DC6E85C6B`.
  Exactly one process at this checkout path exposed exactly one responsive
  `Syndocal` window and was verified maximized. It was then terminated by exact
  resolved path before the final tests. This does not claim normal installer/
  updater inspection or the separate show-ASIO artifact.
- No MOTU M4 stream, M32/DL16 route, or USB-DMX physical output is claimed yet.
  This machine currently exposes no MOTU M4 device and no serial port, so those
  physical rows must be run on the equipped show system. The working tree
  remains uncommitted and unpushed at parent
  `8153ebb37a9517aad91c0da6ad06de9a80db2a1a`.

## Accepted source boundaries

### Pedals and DJ Link source boundaries (not physical acceptance)

- Pedal 1 / F13 owns the current loop toggle in Timeline-control mode.
- Pedal 2 / F14 owns loop-half in both modes: Rekordbox MIDI loop-half in DJ-control mode and `DJ_TIMELINE_LOOP_HALF` for an active Syndocal Timeline loop in Timeline-control mode.
- Pedal 3 / F15 owns Timeline `+4 bars` only in Timeline-control mode.
- F13 DJ release starts HPF and emits the correlated `DJ_RELEASE` on the same edge. The local Rekordbox action then completes HPF, ChannelFader fade, stop, and reset independently of Syndocal delivery.
- Stage 2 commands require exact Timeline/play-session/release authority and revalidate current playing Timeline identity in the engine worker before mutation.
- Re-enabling a completed loop after position B re-enters at A; disabling does not jump.

Focused source evidence on the current dirty tree:

- protocol DJ Link: 14 passed, 0 failed, first-party warnings 0.
- engine DJ Link: 30 passed, 0 failed, first-party warnings 0.
- Syndocal DJ Link: 119 passed, 0 failed, 1 ignored live-network test, first-party warnings 0.
- Independent Stage 2 adversarial review: GO.
- DJ Agent v1.1.10 independent adversarial review: GO. Supervisor focused rerun: 88 passed, 0 failed; `git diff --check` passed before commit.

### Imported fixture stage layout

- `960 sound waves strongpoint` is authoritative as four physical cells and four logical RGB segments. Daslight's twelve displayed cells are the known three-row duplication bug, not twelve physical emitters.
- The Strongpoint collapse requires exact normalized profile identity, twelve raw DVC cells, four logical segments, and the exact three-row duplication pattern. A fixture label cannot trigger the exception; unrelated twelve-cell profiles remain twelve cells.
- Mega Bar remains eight physical cells and eight logical RGBA segments, with separate global dimmer and strobe controls.
- Physical-only layouts remain physical-only; the frontend does not infer color roles from control names.
- Unknown persisted layout fields and duplicate PATCH beam indices fail closed.

Focused source evidence on the current dirty tree:

- protocol stage-layout validation: 13 passed, 0 failed, first-party warnings 0.
- DVC stage-layout unit tests: 5 passed, 0 failed, first-party warnings 0.
- duplicate PATCH beam rejection: 1 passed.
- synthetic exact topology import: 1 passed.
- operator-owned `C:\Users\kouty\Desktop\INMDAISUKI\DSF2026.dvc` acceptance: 1 passed with Strongpoint 4/4, Mega Bar 8/8, and wristband layout absent.
- TypeScript build, Strongpoint browser contract, mapping viewport conformance, stage-label, mapping geometry, and fixture live-color checks: passed.
- Independent physical-layout adversarial review: GO.

### Show authoring and safe publication

- The DSF2026 authoring tool uses a Windows fail-closed safe-write path with exclusive create, reparse rejection, identity revalidation, flush, exact-length verification, and cleanup on failure.
- Independent safe-write review: GO; focused tests 3 passed.
- Fresh native import/save evidence uses the exact operator-owned source `C:\Users\kouty\Desktop\INMDAISUKI\DSF2026.dvc`. The report is exact at `fixtures=46`, `profiles=12`, `fixture_groups=15`, `cues=2`, `Converted=93`, `Approximate=4`, `Skipped=0`, and `Unsupported=1` for one Daslight hardware binding. The saved base has exact stage layouts: six Mega Bar layouts with 8 cells and 8 logical segments each, plus three Strongpoint layouts with 4 cells and 4 logical segments each.
- Historical alpha.27 Native Save As output artifact: `target/qa/dsf2026-native-alpha27/DSF2026-imported-alpha27.sdc`, `1,079,564` bytes, SHA-256 `B21165A70A41A4036153359E579E1433C2739EC1C3EDCC0C46B94F513238DFB1`.
- Alpha.15 is retired as a current authoring base. Its artifacts remain historical evidence only and must not be supplied to the pinned authoring CLI.
- The former alpha3-alpha8 candidates are superseded. The reviewed final candidate is the alpha9 reference-audio artifact documented below; do not deploy an earlier candidate.
- The current supervisor rerun passed the authored-show test, DJ Link frontend/runtime contract, localization `3556/3556` with 0 unprotected labels, stage-label contract, fixture-limit degree contract, Timeline context-menu browser contract, I/O disclosure scroll contract, Strongpoint segment browser contract, Stage Settings viewport contract at five sizes, TypeScript/Vite production build, release metadata, Tauri wrapper self-test, and the `frontend-typescript-vite-windows` warning ratchet. First-party warnings were zero.
- Historical alpha.27 native release evidence: the exact MSVC 14.44 linker gate produced `target/release/syndocal.exe` at `60,314,624` bytes with SHA-256 `CEBB44C713043CCE885D87E3651464F3756A5CEE2D1700728D412AC7B18C48EC`.
- Historical alpha.27 process evidence: exactly one checkout-owned process was responsive after that build, PID `87732`, HWND `124064278`, and the verified Syndocal window was maximized. This is not alpha.28 native acceptance.
- This handoff claims no DJ HELLO/ACK exchange, physical pedal acceptance, or physical DMX-output acceptance; those remain unverified.
- `target\qa\dsf2026-show-authored-20260828\DSF2026-show-alpha3.sdc` passed the current structural Timeline preflight, including the one-measure transition and indefinite pedal-release holds.
- That alpha3 artifact is not the final show file because its base predates the newly imported fixture `stage_layout`. Do not deploy it as the final stage-layout authority.

### 2026-08-28 reference-audio authoring delta

- The two operator-supplied MP3 files are now accepted only as rehearsal/reference material through the ordinary Media Library -> Timeline Audio Clip path. They are not click/guide assets and are not armed for show playback.
- The authoring helper preserves both originals, copies byte-identical files beside the generated candidate under ASCII names, records SHA-256/byte size/duration, and refuses overwrite or divergent existing bytes.
- Each authored song Timeline receives one ordinary Audio layer with one `media_asset_id` clip. The layer is muted by default and requires an explicit operator unmute before rehearsal playback.
- Final reviewed candidate `target/qa/dsf2026-show-authored-20260828/DSF2026-show-alpha9-reference-audio.sdc` is `1,092,555` bytes with SHA-256 `E53AB3B4432E21C8EAEA4F9D727F3AC6ED8793B8282AA10CAEB3A265FDDC3CBF` and passed structural preflight. Measured durations are `214032 ms` for 人生オーバー and `273432 ms` for 惑う星; source and managed-copy SHA-256 values match.
- Windows publication holds verified non-reparse parent/leaf handles across source and sidecar revalidation through candidate flush. Parent/leaf substitution, same-hash reparse substitution, partial write, and flush failure fail closed without candidate or temporary-directory residue.
- Focused authoring regression passed with first-party warnings 0, including explicit no-reference assertions for zero Timeline Audio Clips and zero added Media Library assets. The final independent Terra xHigh review is `GO` with no P0/P1.
- Alpha9 loaded successfully into the exact native process through the single-instance project-forwarding path and reported 46 fixtures plus the expected 12 embedded profiles and mappings. Timeline Audio Clip playback remains unverified.
- A cold command-line launch with alpha9 exposed an open startup race: `Project authority changed before mutation (expected epoch 0 revision 0)`. Starting Syndocal first and forwarding the project path then loaded successfully. This race is not accepted or hidden; fix it in the next narrow tranche before cold-start deployment is claimed.

### 2026-08-28 machine-local USB-DMX delta

- The project retains only the logical serial DMX route. COM/PnP identity is stored in machine-local state and is selected from a current-device dropdown, then persisted only by an explicit Confirm action.
- Missing, stale, ambiguous, renumbered, and A->B->A physical identity changes fail closed. The exact opened Windows handle identity is revalidated rather than trusting a selected port label.
- The S0 blackout epoch is linearized ahead of activation/live ticks. The same physical write gate is held from selected-frame authority through BREAK, MAB, `write_all`, and `flush`; S0-first therefore permits zero live frames, while worker-first permits at most the already-started live frame followed only by zero frames.
- The deterministic worker proof uses the real `EnttecOpenDmxSender` worker and real Open-DMX write sequence with a fake `SerialPort`, including the public `safety_blackout_engage_published` interleave and non-deadlock assertion.
- Focused Rust evidence: `cargo check -p engine`, serial DMX 14/14, engine DMX 29/29, show route 4/4, machine binding 3/3, serial A->B->A 1/1, first-party warnings 0. Frontend output-control, production build, and exact command routing/invoke gates passed.
- Final independent adversarial review is `GO` with no P0/P1/P2. Physical USB-DMX/fixture output remains open.
- Final source gates after the active/bank normalization repair: exact MSVC 14.44 workspace Rust `2683 passed / 0 failed / 17 ignored` across `2700` executed tests, followed by the final exact-gated Syndocal rerun `1199 passed / 0 failed / 12 ignored` across `1211` tests; frontend production build; release/ASIO packaging `169` assertions; Tauri wrapper `231` assertions plus `27` hostile fixtures; exact frontend route/invoke inventories; Stage label, Strongpoint, fixture-limit degree, and remote disclosure contracts. First-party warnings remained 0; `cargo fmt --all -- --check`, `git diff --check`, and `pnpm --dir app run check:release` passed.

### ASIO PROGRAM/CUE output gate added 2026-08-28

- `qa/ASIO_PROGRAM_CUE_OUTPUT_ACCEPTANCE.md` is now the authoritative output gate. ASIO output is a show-critical requirement, not optional polish.
- Adopted architecture C: retain the exact input/Reactive Capture ABI/schema v2 surface, and add an exact v3 output/full-duplex surface to the same canonical bridge DLL. ASIO playback uses one v3 session; a parallel v2 session on the same driver is forbidden and must fail busy.
- PROGRAM stereo and CUE mono are project-level logical buses. Physical PROGRAM L/R, CUE, optional Spare, driver identity, sample rate, format, and buffer remain machine-local. Missing bus data migrates one way to PROGRAM; CUE never falls back to PROGRAM.
- Two logical Rodio mixers feed a non-realtime renderer and bounded preallocated interleaved SPSC. The ASIO callback only copies a complete block or outputs a complete silent block and latches terminal Fault. PROGRAM/CUE share one device and clock.
- DSF2026 acceptance mapping is MOTU M4 Output 1/2 = PROGRAM L/R and Output 3 = CUE at exact 48 kHz, with Output 4 optional Spare. This is a selectable profile, not MOTU-specific code.
- Device/rate/mapping conflict, disconnect, XRUN, reset/resync, buffer/rate change, callback gap, or underflow must stop output without WASAPI/default-device/rate fallback. Explicit revalidation and Start are required.
- Independent Terra xHigh review added three implementation-blocking P0 boundaries: quiesce and join the legacy `FollowProgram` CUE/normal Rodio output before v3 Start and stay silent on Start failure/Fault; freeze exact v3 callback/queue/lifetime semantics before code; and prove arbitrary non-contiguous/reordered physical mappings with all unselected channels zero. It also requires exact queue/race injection tests and updates every show-ASIO v2-only export checker to the exact v2-nine plus v3-nine set.
- The acceptance contract was committed and pushed at `e583141cc60decff7c062db21a39f69241f894c8`. The active implementation checkpoint is `1.2.0-alpha.28` on top of the alpha.27 source/native checkpoint and remains uncommitted pending production integration and exact acceptance gates.
- Integrated source, independent review, and the exact normal no-bundle native/window gate are complete in the current checkpoint. Dedicated show-ASIO artifact/loader proof, normal installer/updater inspection, authoritative non-default Timeline speed synchronization, MOTU M4 output proof, and M32/DL16 routing proof remain open.

### 2026-08-28 alpha.28 ASIO implementation checkpoint (superseded source snapshot)

This subsection preserves the earlier partial snapshot. The 2026-08-29
integrated source checkpoint above is authoritative for current source status.

- Source authority remains branch `codex/syndocal-v1.2` at committed/pushed parent
  `8153ebb37a9517aad91c0da6ad06de9a80db2a1a`. The `1.2.0-alpha.28` source below
  is a shared uncommitted working checkpoint; production code integration,
  native acceptance, and physical acceptance remain pending, so no deployment
  or physical acceptance follows from it.
- Portable Timeline Audio Clips now carry only `PROGRAM | CUE`. Missing legacy
  values become PROGRAM, explicit invalid/future values reject, and root,
  child, import, save/reload, split, lane move, and sink source identity retain
  the logical bus. A stale full authoring/bank publication cannot overwrite a
  newer dedicated bus mutation. Bank preservation is keyed by
  `(TimelineId, clipId)`, so two Timelines that reuse a clip ID cannot
  contaminate one another. Independent Terra xHigh rereview is GO for this
  engine boundary; exact MSVC 14.44 focused evidence is 15/15 Timeline audio,
  1/1 strict protocol bus rejection, 1/1 root and child invoke-wire checks,
  1/1 sink retirement, and 11/11 static UI assertions, with first-party
  warnings 0.
- The v3 bridge is an actual ASIO SDK output/full-duplex implementation, not a
  CPAL callback placeholder. It owns explicit driver load/init, exact tuple
  negotiation, one `ASIOCreateBuffers` set, output/full-duplex callbacks,
  `ASIOStart`, terminal faulting, and drained Stop/Close. Full-duplex input and
  output must share the exact sample rate and fixed buffer before driver open;
  both mismatch directions reject without a handle, session, result, or false
  `actualInput`. Independent Terra xHigh rereview is GO with P0/P1 0. Final
  bridge evidence is default 31/31, ASIO 34/34 plus one physical ignored test,
  clippy `-D warnings` in both graphs, and exact v2 nine plus v3 nine exports.
  The current bridge DLL is 926,208 bytes with SHA-256
  `B21713C5C3F4A3ECA1C3A12769E05076136A43D40D7FBF225DDE525D6792A294`.
- The isolated `AudioOutputRouter` core received independent Terra xHigh GO.
  It provides process-wide ownership, sealed/ticket-bound source creation and
  recovery, late-prepare retirement, retryable exact resource teardown,
  monotonic callback drain, explicit Ready cancellation and return to Normal,
  and fail-closed identity exhaustion. Its standalone proof is 12/12 with
  first-party warnings 0. It is not yet a claim that the production app has
  routed every Rodio constructor through the owner.
- Prior focused ASIO preflight evidence is 17/17, 11/11, and 15/15, with
  first-party warnings 0. This is preflight/source evidence only; it does not
  close production code integration, the alpha.28 native build, or hardware
  acceptance.
- The show artifact source identity now includes the application owner/cue
  runtime, both v2/v3 loaders, router and tests, render/runtime modules, bridge
  build script, shared lease, ABI/native/SDK FFI and C++ callback sources,
  headers, tests, manifests, and trusted helpers. Critical runtime mutation is
  rejected before DLL inspection. The checker self-test currently passes 53
  assertions; the exact identity count is 55 after adding the split app render
  and runtime modules, the normal-output lease, the compact Audio control/UI
  integration and its focused checkers, plus the complete fifteen-file bridge
  identity.
- Current SDK-independent alpha.28 gates: TypeScript/Vite production build PASS
  (`302` modules), and `check:release` PASS including synchronized release
  metadata, ASIO packaging `169` assertions, v3 contract `22`, Timeline
  PROGRAM/CUE bus `11`, and existing video route/window gates. These checks do
  not establish complete production integration or native/hardware acceptance.
- Still open and release-blocking: production app code integration, including
  v3 lifecycle wiring, two
  Rodio mixers drained by one non-RT worker into a preallocated bounded SPSC,
  app-owned mutable transport generation, complete normal/FollowProgram/
  ExplicitDevice quiesce, machine-local IPC and compact Audio UI, independent
  app/UI review, exact normal and show-ASIO native builds, real loader smoke,
  and all MOTU M4/M32/DL16 physical rows.
- Cleanup inventory was read-only: `target` 364,735,087,090 logical bytes
  (339.69 GiB), `app/node_modules` 545,338,492 bytes, bridge `target`
  1,778,080,767 bytes, and `app/dist` 5,131,553 bytes. The accepted recurring
  deletion conditions are not currently satisfied, so no cleanup Apply or
  ad-hoc deletion ran and reclaimed bytes remain 0.

## Required remaining acceptance

1. Freeze, commit, and push the integrated `1.2.0-alpha.28` source checkpoint and this evidence. No dedicated show-ASIO artifact may use the current dirty tree as source authority.
2. Build and inspect the normal NSIS/MSI/updater artifacts so the complete default-distribution ASIO-free gate is measured, not inferred only from source packaging tests.
3. Build and verify the exact local-only show-ASIO artifact from the clean pushed source commit: exact 18 exports, v3 S/E/B manifest and source hashes, real loader Start/Stop/Fault smoke, and one responsive maximized Syndocal window.
4. Implement and prove authoritative non-default Timeline speed synchronization for PROGRAM and CUE; the current explicit rejection is fail-closed but does not complete the acceptance row.
5. Pull the committed DJ Agent checkpoint on the DJ PC without exposing the token and confirm strict preflight, active runtime version, real ACK, reconnect snapshot recovery, and physical Pedal 1/2/3 behavior.
6. Verify serial DMX through the operator-selected actual USB interface and physical fixtures. Do not assume a fixed COM number on the show PC, and serialize output ownership against Daslight without terminating Daslight implicitly.
7. Perform MOTU M4 at exact 48 kHz and M32/DL16 physical acceptance. PROGRAM must reach only M4 1/2 -> DL16 5/6 -> M32 Ch18/19 -> Main/Broadcast; CUE must reach only M4 3 -> DL16 7 -> M32 Ch20 -> IEM 4/5/6 and remain absent from Main/Broadcast/Floor.
8. Update this handoff with physical evidence and exact artifact identities. Hardware, real ACK, serial DMX, ASIO device, M32 routing, reconnect, installer/updater inspection, and dedicated show-ASIO acceptance remain explicitly unverified until observed.

## First safe resume actions

- Do not regenerate the final show from alpha3 or deploy superseded alpha4-alpha8 reference candidates; alpha9 is the reviewed reference-audio candidate.
- Keep the current Syndocal/Rekordbox/DJ Link processes alive until immediately before the exact alpha.28 native release build boundary.
- Re-run `git status --short`, verify branch/HEAD/upstream equality, and inspect every owned diff before versioning or committing.
- Preserve the operator-owned DVC, all token material outside the checkout, and existing QA artifacts.
