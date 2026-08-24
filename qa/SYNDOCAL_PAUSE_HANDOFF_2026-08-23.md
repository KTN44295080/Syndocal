# Syndocal pause handoff — 2026-08-23

> Historical checkpoint only. It was the authoritative resume point for the
> 2026-08-23 pause. The later post-alpha.10 snapshot also became historical when
> the operator rescinded that requested pause before promotion on 2026-08-25.
> Preserve the exact evidence below; current work follows `AGENTS.md` and
> `qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md`.

This document records the intentionally paused Windows completion run, its then
unfinished integration, and its then-current product checklist. It does not mark
unverified work as accepted.

## 1. Stop state

- Branch: `codex/syndocal-v1.2`
- HEAD: `23f350c366ede2fdffcfbf3232e18112eada51ea`
- Historical stop-state snapshot at this checkpoint: current product metadata was
  `1.2.0-alpha.7`.
- Next native development artifact: `1.2.0-alpha.8`; do not overwrite or relabel
  the crashing alpha.7 artifact.
- Accepted active denominator before this pause: 19/71 (26.8%). D3, the Tauri
  mitigation, DJ transport, and hardware work below do not advance it yet.
- The worktree is intentionally dirty and shared. Do not reset, checkout, clean,
  or discard any listed change. No commit or push was made at this pause because
  the integrated D3 and native gates are incomplete.
- No alpha.8 native build was produced. No new native process or hardware action
  was started after the pause request.

Modified/untracked scope at the stop:

- Root/dependency: `Cargo.toml`, `Cargo.lock`, `vendor/`
- Native backend: `app/src-tauri/src/main.rs`, `control_plane.rs`
- Engine/I/O: `crates/engine/src/lib.rs`, `crates/io/src/remote_ws.rs`
- Frontend and routing: `app/src/App.tsx`, the fixture/PATCH panels, output/video
  controllers, invoke manifest/generated registry, and focused checkers
- QA: Tauri mitigation note and native-physical evidence directory
- Product-version/document files from alpha.7 remain dirty and must be advanced
  together only when alpha.8 is actually built.

## 2. Completed evidence that may be reused

### 2.1 D3 frontend and atomic Cue operations

- D3 frontend fixed-hash independent review returned P0/P1/P2 zero.
- Atomic single-dispatch commands replace the invalid repeated-dispatch ticket
  flows: `update_cue_from_current_batch` and
  `move_cue_between_scene_banks_batch`.
- Scene-bank placement uses exact `targetCueId + before|after`; a null target is
  normalized to `after`/destination end. Browser/native ordering equivalence and
  publication rollback have focused proof.
- Frontend routing inventory is 133 renderer-ticketed and 29
  backend-authoritative routes; frontend invoke inventory is 417.
- Final frontend hashes before this pause are recorded in
  `qa/CODEX_HANDOFF_2026-08-19.md` and the agent handoff. Do not regenerate the
  manifest or checker counts until the backend 478-command registry is stable.

### 2.2 Tauri `Context` clone crash mitigation — static approval only

- Vendored `tauri-runtime-wry 2.6.0` backport matches the upstream PR 15411 Weak/
  Arc ownership, main-thread upgrade, display-handle routing, and drop order.
- Ox Max (`opencode/x-preview-f-free`, variant `max`) independently compared the
  crates.io source, upstream commits, unsafe boundary, Cargo patch/lock delta,
  and warning compatibility. Static verdict: P0=0, P1=0.
- Candidate hashes before the later product-version bump:
  - `Cargo.toml`: `001D6D11D02FF9B3DEE5FD1C9B25F88686F6A723FE4692A4454F8029720BC1E2`
  - `Cargo.lock`: `43D0932D5EDB57A40A1C861266789E644F03CC3AA84DDDAE07700BEDEB85421E`
  - vendored `src/lib.rs`:
    `ABFC5DEEB94E38C70FDAF5C65DFC811B304E9CBFEE29040DCC63D5185BD9A154`
  - provenance note:
    `8219195838DC4BEE3B9BBE4B094683340781416F752AFCB5B45103D6159443EE`
- Vendor `--locked --all-targets` and Syndocal no-default Cargo checks passed with
  zero warnings on that snapshot.
- This is not native acceptance. The prior alpha.7 executable repeatedly produced
  WER `0xc000001d`; only the alpha.8 native stress matrix can close it.

### 2.3 DJ transport current source checkpoint

- `crates/io/src/remote_ws.rs` stable SHA-256:
  `82CC17B68278385DB119D0C77519C936F388F13CB021808ADCEE53AD71CFD232`.
- Physical `(agent,event)` tombstones are compact keyed digests, capped at 262,144
  identities. Existing identities are never evicted; capacity permanently latches
  new physical identities closed until process restart. Nonphysical traffic can
  continue.
- The cap covers more than one hour at the default 60 messages/s even if every
  accepted frame is physical. A higher configured rate can latch earlier; this is
  a documented availability boundary, not silent eviction.
- Terminal TTL is purged on real admission. Same physical intent after receipt
  expiry returns `ReplayNotRetained`; changed type/payload returns `Conflict`.
- Restart clears the process-local fence. Cross-process exactly-once is not claimed.
- `cargo test -p io --lib`: 135 passed, 0 failed, 1 physical-MIDI ignored;
  first-party warnings 0; io fmt and targeted diff-check passed.
- Independent Terra review of the predecessor found the unbounded-memory and TTL
  defects now addressed. The new fixed hash still requires a fresh Ox Max or
  equivalent independent read-only review.

### 2.4 D3 backend partial source freeze

- `main.rs`:
  `A049518387D2064E065DE616C498B7D50DFC9AF68E178A8942B3FE88165F92F3`
- `control_plane.rs`:
  `74D93D27A8B1C2D4E37A9262D2C013EC1D49A89D2DE920BFE3945D0C7369EF51`
- `engine/src/lib.rs`:
  `0862FEEC64C5D5A7F6307EF171836C9CD353B1B4F95CF2AABB0CC984D2E75770`
- The self-admitted retry carveout is restricted to the two routes with an exact
  request digest and retained terminal result: `patch_fixtures` and
  `repair_fixture_profile`. The other 11 routes are one ticket/one attempt.
- Runtime dispatch is exhaustively divided into 53 synchronous direct routes that
  retain the external gate through handler completion and 89 preflight-only routes
  with their own inner authority/CAS, pure preview, or machine-only behavior.
- Focused exact-two, one-attempt, Runtime partition/interleaving, FileExport, fmt,
  and targeted diff checks passed on the source lineage.
- This is a partial freeze, not an accepted D3 checkpoint. The final D2 reply-loss,
  control-plane suite, no-default check, full fmt, warning-zero proof, and fixed-
  hash independent review were blocked while `remote_ws.rs` was moving.

## 3. Historical exact resume order — do not execute

Do these in order. Do not start hardware or an alpha.8 build before steps 1-5 are
green.

1. **Independent DJ transport review**
   - Review only SHA `82CC17B...D232` read-only.
   - Recheck compact digest collision behavior, cap/inflight accounting, repeated
     capacity rejects, TTL admission purge, reconnect/high-sequence replay,
     Heartbeat/nonphysical exclusion, terminal ACK truth, restart boundary, and
     production `/dj-link -> dispatch_dj_link_event -> Engine` routing.
   - Rerun io tests/fmt/diff and confirm start=end hash.

2. **Integrated D3 backend gates**
   - Re-run exact-two receipt/retry and generic one-attempt/concurrent tests.
   - Re-run D2 production reply-loss/one-publication proof, atomic Scene-bank/Cue
     batch tests, project-transaction tests, all 26 control-plane tests, external
     admission ordering, and FileExport/Runtime partition tests.
   - Run no-default Cargo check and the supported warning configurations; require
     zero first-party warnings in all modified files.
   - Run workspace/package fmt and diff-check. Do not suppress warnings.

3. **Final D3 fixed-hash adversarial review**
   - Verify exact-two receipt carveout really checks payload digest and terminal
     result, while every nonreceipt route rejects a second/different payload.
   - Try owner/window/incarnation/epoch/command substitution, Full/Partial lock,
     project replacement between admission and dispatch, reentrant/deadlock paths,
     long I/O while gated, async worker inheritance, reply loss, and raw Tauri
     invocation.
   - Require P0=0/P1=0 and no release-blocking P2 before checking D3 complete.

4. **Re-run the frozen frontend gate chain**
   - `tsc --noEmit`, production build, warning ratchet, routing checker, invoke
     inventory, backend operator contract, transaction/recovery/publication checks,
     output ownership/runtime checks, Scene Matrix five-viewport/14-check gate,
     workspace/operator/DJ/profile/fixture/storage/shortcut checks, and diff-check.
   - Confirm exact 478 backend commands, 133 renderer mutations, 29
     backend-authoritative routes, and 417 frontend-used invokes.

5. **Integrated source freeze and documentation refresh**
   - Record exact hashes for every owned backend/frontend/I/O/vendor/checker file.
   - Update D3, Tauri, and DJ acceptance sections without changing the 19/71
     denominator until native/hardware evidence is actually accepted.
   - Remove generated build artifacts from `vendor/` before commit; retain only
     reviewed source/provenance/license material.

6. **Advance the artifact to `1.2.0-alpha.8`**
   - Synchronize workspace Cargo version and the eight first-party lock entries,
     app package, Tauri config, release checker, artifact/workflow names, README,
     completion flow, roadmap, and handoff.
   - Run `pnpm --dir app run check:release` and record zero-warning/diff evidence.

## 4. Required alpha.8 native crash and UI acceptance

1. Resolve every running process whose executable path is exactly this checkout's
   `target/release/syndocal.exe`; terminate only that exact path.
2. Run `pnpm --dir app tauri build --no-bundle` successfully with the supported
   FFmpeg/LLVM/MSVC environment and record first-party warning counts.
3. Hash the exact EXE/PDB, verify ProductVersion/FileVersion alpha.8, launch it, and
   prove exactly one responsive `Syndocal` window. Maximize that verified window
   before every normal UI action.
4. Establish a fresh WER baseline for the exact executable.
5. Exercise at least 100 WebView reloads while async queries/events are active.
6. Repeat trusted-LAN/DJ config toggles, DJ Link interface enumeration, Web Remote
   URL refresh, maximize/restore/F11, and rapid open/close patterns that previously
   hit Tauri `Context<EventLoopMessage>::clone`.
7. Confirm the exact PID remains alive/responsive and no new `0xc000001d` or other
   WER event belongs to the alpha.8 executable. If it recurs, capture ProcDump/WER
   dump and symbolicate against the exact PDB before any UI workaround.
8. Native UI screenshots/geometry at the actual supported classes available on
   this host: 1280x720, 1920x1080-class, and 2560x1440. 3840x2160 may remain browser
   evidence unless made an active verified desktop. Do not revive 860x520 as a
   completion requirement. Keep 960x640 only as the configured minimum boundary.
9. Reverify Lighting Scene Matrix, Video Media Library/Import, Timeline normal and
   expanded, Tools/Live Mixer, Sources/Inspector height and scrolling, Escape/focus
   order, popup last-action reachability, and lower-band inertness.

## 5. Required Windows real-operation and hardware acceptance

### 5.1 Remote and DJ Link

- Start/stop Web Remote, verify final bind URLs and no stale async overwrite.
- Launch the DJ Agent from `C:\Users\kouty\Desktop\rb-output` only after alpha.8
  native crash closure. Use the dedicated `/dj-link` route, not `/ws`.
- With rekordbox and the Stream Deck Pedal, record preload/non-Master non-trigger,
  Master Track start, absolute/repeated loop divisions, Filter isolation,
  idempotent Release, State Sync/ACK, local operation during disconnect, reconnect,
  and next-use readiness.
- Stress replay/ACK expiry, agent reconnect, invalid token/origin/host, high-rate
  capacity latch, and process restart. Do not claim cross-process exactly-once.

### 5.2 Lighting and control I/O

- Use the connected COM serial device with its actual adapter/protocol; verify DMX
  addressing, RGB/wheel, Pan/Tilt, intensity, strobe, Blackout, multi-universe,
  disconnect/reconnect, and continuous output.
- Complete Enttec USB PRO/DMXKing/Open DMX timing where the corresponding hardware
  is present; record Break/MAB/frame period and fail-closed removal.
- Test physical MIDI Note/CC/Clock/MTC, feedback, All Notes Off, mapping replacement,
  stale callback retirement, disconnect/reconnect, and OSC/Touch paths where
  available.
- Complete RDM/TOD discovery/correlation/timeout/cancel/removal if it remains in the
  supported Windows artifact.

### 5.3 VJ/video/displays

- Exercise actual VJ Program output with dual display/HDMI/fullscreen, DPI/refresh,
  unplug/reorder, GPU reset, and output ownership/re-Arm.
- Test NDI and Spout on Windows; camera/screen/generator sources must report truthful
  availability, permission, reconnect, and retired-worker state. Syphon remains a
  non-Windows/deferred route.
- Run maximum-layer/ISF/Preview/Program/recording load with frame/drop/CPU/GPU/RAM/
  VRAM logs and operator-response measurements.

### 5.4 Audio, Guide, and ASIO

- Verify audible click and Guide routing on the selected real device. The accepted
  deterministic Guide boundary is target minus 7,200 frames (150 ms at 48 kHz),
  but that does not prove audible output.
- Complete WASAPI device/rate/channel/buffer/fault testing and one-hour soak.
- Keep the non-default licensed ASIO bridge separate from the MIT/WASAPI artifact.
- Select and document the ASIO distribution/license path.
- Complete 44.1/48/96 kHz, 64/128/256 frame, format, mono/stereo/channel matrix;
  occupied driver, control-panel change, reset/resync, XRUN, unplug/replug,
  callback-gap/no-callback recovery; selection persistence and stale/ambiguous lock.
- Run matched one-hour ASIO/WASAPI soak and required callback/capture/engine
  percentiles, physical input-to-pixel latency, and five matched TouchDesigner
  trials. Package telemetry must prove no silent fallback.

### 5.5 Disconnect, recovery, and integrated soak

- Rehearse New/Load/Recovery/Backup/Take Over retirement and explicit re-Arm with
  acknowledged physical output.
- Exercise device/network/DJ Agent/Syndocal crash and restart, cable removal,
  partition/rejoin, and stale-generation rejection.
- Run at least one hour with the final exact alpha.8 artifact and the intended
  Lighting + VJ + MIDI/MTC + DJ Link + Pedal + audio/ASIO combination. Record
  process responsiveness, WER, output continuity, dropped frames, audio overruns,
  callback latency, CPU/GPU/RAM/VRAM, and recovery actions.

## 6. Remaining implementation after the immediate Windows run

These are still open in the master completion flow even if the immediate alpha.8
hardware run passes.

### Phase 0 / evidence discipline

- [ ] Build complete Q1-Q4 traceability, decision, risk, and evidence coverage.
- [ ] Extend `check:release` with tag, previous-version, updater, and artifact checks.
- [ ] Maintain zero first-party warnings and remove remaining debt without broad
  suppressions or fake reads.

### Phase 1

- [x] Accept D3 after the integrated gates and fixed-hash review above.
- [ ] Implement and accept D4 Stage import/mutation identity, whole-operation
  atomicity, Undo, and truthful errors.

### Phase 2 / AI3

- [ ] Add durable crash-safe terminal recovery; do not present process-local
  receipts as durable truth.
- [ ] Verify all ingress policies with native clients/hardware.
- [ ] Close project replacement/re-Arm, dangerous-action dialog, physical
  creation/teardown ACK, and five-display hardware behavior.

### Phase 3 / AI0-AI8

- [ ] AI0 exhaustive source inventory with unknown mutation fail-closed.
- [ ] AI1 query/event schemas, generations, gaps/resnapshot, and bounds.
- [ ] AI2 authored command bridge with E/R/H, owner incarnation, receipts, Undo.
- [ ] AI4 principals, grants, revocation, kill switch, consent, and bypass absence.
- [ ] AI5 authenticated sidecar/MCP/API/WebSocket/discovery and bounded lifecycle.
- [ ] AI6 administration UI.
- [ ] AI7 adversarial parity/security/rate/reply-loss/fuzz/saturation proof.
- [ ] AI8 native external-client, clean-install, hardware, crash/restart, and
  security acceptance.

### Phase 4 / ownership and ShowClock decisions

- [ ] F1 monotonic generations and stale MIDI/OSC/DMX worker retirement.
- [ ] F2 full Lighting/Video/Both/Standby ownership across physical/native outputs.
- [ ] Freeze ShowClock transport, authentication, replay, estimator, Hold/slew,
  witness/fence/interlock, mixed-version, and supported-network decisions.

### Phase 5 / show features

- [ ] Reintegrate accepted Clip Slot/Layer Bus/FX work into the full gate.
- [ ] C2 Clip Take and C4 mapping/Timeline integration.
- [ ] Timeline Follow/crossfade, BPM slew, failure policy, and Trans/Complete Guide.
- [ ] Timeline Undo/Redo/save/reload focus, fixed Guide device, native A/V/Lighting
  synchronization, fault, and viewport proof.
- [ ] Media thumbnail/waveform/proxy/analysis identity, bounded workers,
  cancellation, cache/eviction, predecode/prefetch/degraded operation, and budgets.
- [ ] Authored Audio schema/migration/history and clock/resample/seek/loop/underrun/
  device-fault semantics.
- [ ] Live camera/screen/NDI/Spout/generator identity and fault/reconnect truth.
- [ ] Recording state machine, target reservation, crash/disk/encoder recovery,
  atomic artifact, two-PC ownership, and authoritative asset import.

### Phase 6 / product completion

- [ ] Implement ShowClock simulator, authenticated peer sync, timestamped
  exactly-once actions, coupled generations, witness/fence, UI, and two-process/
  two-machine fault proof.
- [ ] H1-H5 reachability/shared shell/Setup/Edit/Control completion with native
  evidence.
- [ ] DJ Link Setup NIC/bind/token/session/current-track/mapping/recovery UI.
- [ ] Remote/Touch LAN/TLS, pairing, Origin/Host, grants, rate/size, RDM, parser and
  archive security.
- [ ] Supported schema/version matrix, migration corpus, corrupt/hostile input,
  backup/recovery/upgrade compatibility.
- [ ] Diagnostics, redaction, updater channel/downgrade/signature/offline/rollback,
  and operational runbooks.
- [ ] NVDA, High Contrast, color independence, 125/150/200% scaling, keyboard-only
  safety, IME, focus, and reduced-motion acceptance.

### External/comparative and final release

- [ ] Real Art-Net/sACN nodes/fixtures and physical DMX continuity/timing.
- [ ] Dual-display/VJ/live-source/venue one-hour acceptance.
- [ ] Two-machine real-switch crash/partition/rejoin rehearsal with zero simultaneous
  output.
- [ ] Pinned Daslight/SynapseRack/TouchDesigner comparative tasks with measured
  first failures and synchronized evidence.
- [ ] When distribution becomes in scope: supported artifact matrix, signing,
  licenses/SBOM/notices, clean install/upgrade/uninstall, signed updater, immutable
  tag/artifact/evidence publication and rollback rehearsal.
- [ ] Run the complete final verification order: format/tests/check/clippy warning
  zero; all frontend/static/localization/viewport gates; frozen adversarial review;
  exact-process native build/UI/accessibility; full physical/ASIO/performance soak;
  ShowClock/two-PC faults; migration/corruption/security; clean-machine release.

## 7. Historical resume commands — do not execute

- First read this file, `qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md`,
  `qa/CODEX_COMPLETION_ROADMAP_2026-08-13.md`, and the latest handoff before editing.
- Confirm the current diff and hashes before assigning ownership. Never let two
  implementers edit the same file; reviewers remain read-only until a declared
  freeze.
- Preferred Ox invocation, when available through OpenCode Zen:
  `opencode run -m opencode/x-preview-f-free --variant max`.
- Use Sol for integration/hard design, Terra for independent review or bounded
  implementation, and keep the supervising agent responsible for final integration.
- Before native UI automation, select exactly one verified Syndocal window,
  maximize it, observe, then act. Never automate an unrelated `syndocal.exe` or
  terminate Daslight.
- Do not commit/push this paused tree until the integrated checks are green and the
  checkpoint is reviewable. After that, create a meaningful commit, push it, and
  record exact commands/results and every unverified hardware boundary.

## 8. Claims prohibited at this pause

- Do not claim D3 accepted or the raw-Tauri mutation surface fully closed.
- Do not claim the new DJ transport fixed hash independently approved.
- Do not claim the Tauri crash fixed in native operation.
- Do not claim alpha.8 exists, nor that alpha.7 is safe for production.
- Do not claim 2560/1280/4K native UI acceptance from browser evidence.
- Do not claim DMX, MIDI/MTC, rekordbox/Pedal, VJ output, audible Guide, ASIO,
  disconnect recovery, or one-hour soak complete.
- Do not increase the accepted denominator beyond 19/71 without the corresponding
  accepted evidence.

## 9. Historical resume checkpoint — 2026-08-23 supervisor continuation

Branch/HEAD remain `codex/syndocal-v1.2` / `23f350c366ede2fdffcfbf3232e18112eada51ea`.
The shared dirty tree was preserved; no reset, checkout, clean, commit, or push was
performed at this checkpoint. The accepted denominator remains **19/71**.

Verified supervisor evidence so far:

- `crates/engine/src/control_plane.rs` had a stale exact inventory expectation:
  generated `EngineCommand` count 263 versus expected 262 after the existing
  `MoveCueBetweenBanksPublished` addition. The expectation is now 263 and its exact
  test passes.
- `max_loop_pre_wait_and_follow_dispatch_128000_occurrences_without_loss` passed ten
  consecutive isolated runs. Scheduling measured 29.9437–38.9157 ms against the
  unchanged 75 ms ceiling; dispatch measured 101.3–190.7 microseconds against the
  unchanged 25 ms ceiling.
- A full `cargo test -p engine --lib` rerun reached 814 passed / 1 failed / 2 ignored.
  The remaining failure is the new DJ admission test observing `Queued` rather than
  `Admitted` under full-suite scheduler load. This is not waived.
- A full `cargo test -p syndocal --locked --bin syndocal --no-default-features
  --no-fail-fast` rerun reproduced 934 passed / 1 failed / 5 ignored. The failure is
  `recovered_display_project_allows_exact_renderer_ticketed_fixture_group_mutation`:
  after installing a `lock_on_load=true` Partial Operator policy, the test attempts
  to clear it without establishing an unlocked renderer session. The backend rejects
  it truthfully with `Operator Partial Lock blocks project mutations...`; the test
  fixture must explicitly model a successful unlock before the clear operation.
- The current DJ fixed-hash adversarial review is **not accepted**. Confirmed P1
  findings are: State Sync can seed a runtime-only Loop/Release fast path that later
  returns `Accepted` without an Engine convergence; Beat Jump performs the physical
  Engine mutation and can then reject from stale `runtime.position_bars`; and an
  admitted DJ command can wait indefinitely on `snapshot.write()`, contradicting the
  listener's four-second shutdown contract. Final reviewer counts and post-fix hashes
  are still pending.
- The companion DJ Agent at `C:\Users\kouty\Desktop\rb-output` reached a stable
  seven-file snapshot on `Beta` / `815b50e7ae79383b4d618724bf7c03b116e88405`.
  Supervisor rerun: `npm test` 54/54 passed, `node --check` 16/16 passed,
  `git diff --check` passed, and zero first-party warnings. Independent fixed-hash
  review nevertheless returned **FAIL, P0=0/P1=3/P2=2**: inbound flat Timeline State
  was accepting missing/case-normalized/nested fields and could incorrectly arm Stage
  2; typed ACK accepted missing always-present `message`/`code`; and caller-supplied
  control IDs were not reserved against control reuse or later physical collision.
  README also contradicted its own process-environment rule for a nonstandard
  rekordbox path. These are assigned back for source/test correction. `dist` and
  `server.exe` have not yet been rebuilt.

Next action is strict: close the KDMX DJ P1 findings and full-suite failures, obtain a
new fixed-hash review with P0/P1 zero, obtain the companion DJ Agent fixed-hash review,
then begin the documented D3 backend gate order. Native build, product-version bump,
packaging, process replacement, UI automation, and hardware operation remain gated on
those source reviews.

### 9.1 Companion DJ Agent fixed-hash closure and package checkpoint

The companion DJ Agent source re-review is now **PASS, P0=0/P1=0** at `Beta` /
`815b50e7ae79383b4d618724bf7c03b116e88405`. Start and end hashes matched for all
seven assigned files. The reviewer independently confirmed the exact flat seven-field
Timeline State schema, exact mandatory eight-field ACK schema, malformed-frame
non-readiness/pending-until-timeout behavior, caller control-ID rejection, process-wide
monotonic `control-*` IDs across reconnect/client recreation and more than 4096 IDs,
and physical/control-prefix collision rejection. Reviewer reruns were `npm test` 54/54,
focused strict tests 3/3, `node --check` 16/16, and `git diff --check`; first-party
warnings were zero.

After that review passed, the supervisor rebuilt the exact
`C:\Users\kouty\Desktop\rb-output\dist` target with
`scripts\build-dist.ps1`. The old 39-file / 220,457,060-byte tree was first copied to
`C:\Users\kouty\AppData\Local\Temp\rb-output-dist-pre-dj-agent-20260823` so the
destructive rebuild remained recoverable. The build exited zero and produced 39 files /
220,512,937 bytes plus an eight-entry `rb-output-20260823.zip`. Current artifact hashes:

- `server.exe`: `339ECF6E82EB463F55977F63A137CB0CB52886CD7E2874E87F5AD4724234377B`
- `inject_hook.exe`: `E38156656CC75B3C85C1591AF005FFB02FBC4B16A4C27968656196F6C524F9F2`
- `content_lookup.exe`: `EB78F7F078B26CB1762D2C18DC27D4BAA017C2BE2B5CDED64CB8F45946296ACE`
- `rb-output-20260823.zip`: `5562ABD65B874E6A230FD222616E79E146DA31C2686D06EB2567436D085C180B`

The packaged `server.exe` was launched from that exact `dist` directory on isolated
port 8788 with DJ Agent and Hook UDP disabled. `/api/health` returned `ok:true`, `/`
returned HTTP 200, `/api/dj-agent/status` truthfully returned `disabled`, and the smoke
process was stopped only after its executable path was revalidated; port 8788 was
released. PyInstaller repeated its pre-existing optional/platform missing-module list
(content lookup 240 entries, inject hook 14; same counts as the preserved prior build)
and three SQLAlchemy optional-driver console warnings; no first-party warning was
introduced. The pre-existing source service PID 97208 remains running on port 8787
with its original process environment until the KDMX endpoint is ready for a controlled
integration restart. Physical MIDI, Pedal, rekordbox, and Syndocal acceptance remain
unverified and are not claimed here.

The companion README was then updated to replace its obsolete "dist not rebuilt"
statement with only the measured artifact and isolated-smoke facts above. Its new
SHA-256 is `D5D164D60D29C40879190561AFA44D6828648A6F4FB8398035B40B36EA2E8DDA`.
An independent documentation-only re-review returned PASS, P0=0/P1=0, rechecked the
server hash and eight ZIP entries, repeated all three HTTP 200 smoke endpoints on
port 8788, and confirmed that the wording still leaves every physical interoperability
claim open. The smoke process was path-verified and stopped; port 8788 is released.

This independently green companion checkpoint was committed as
`6c4f4328a6866d9d48022bd8ee20a7887c9de851` (`feat: harden Syndocal DJ agent
transport`) and pushed successfully to `origin/Beta`; local HEAD and upstream match
and the companion source tree is clean. GitHub's push response reported six findings
on the repository's default branch, so the supervisor also ran
`npm audit --omit=dev --json` against this exact Beta checkout. It reported 99 current
dependencies and zero low/moderate/high/critical production vulnerabilities. This
does not substitute for the still-open physical/interoperability acceptance.

### 9.2 KDMX DJ transport second fixed-hash review

The first corrective KDMX snapshot reached green implementer gates at Engine
817/817 (two ignored), I/O 143/143 (one ignored), and app no-default 935/935
(five ignored), with zero first-party compiler warnings. Stable hashes were Engine
`A0BD7726EADB9F4C17F0FC6B42D677A8CE92D371CE373434AAB7AE19C733ADC8`, I/O
`20965E36819CEB148A03A9C57EBE68710FEC9FB54FAC9F69EC447F49A49A023D`, and app
`2DF9675D78E405D49B4A9EAD7F37DB59BFA3C26D068B4A114DA0E5C2F8393AFC`.
The independent start/end-hash review nevertheless returned **FAIL, P0=0/P1=2/P2=1**;
this snapshot is not accepted.

- A validated State Sync with `released=false` and an explicit loop division now
  executes an Engine absolute-loop mutation, but I/O still classifies every State
  Sync as nonphysical. It therefore bypasses the process-lifetime fence, inflight
  reservation, capacity latch, and restart retirement. The assigned repair is
  payload-dependent classification fixed at admission and reused through completion:
  only that active-loop quadrant is physical; the other three released/division
  quadrants remain observation-only/nonphysical.
- The Engine's three-second admission wait plus separate one-second grace can return
  a rejected terminal while an admitted worker later publishes B. It also consumes
  the entire advertised four-second listener-stop budget before join/socket overhead.
  The assigned repair is a four-API cancel-or-commit receipt with one absolute
  deadline: cancellation must win before commit and restore exact A with no late
  mutation, or commit must win and produce the only Accepted result. Caller-only
  timeout and worker detach are forbidden.
- `dj_link_engine_transition_broadcast_is_async_and_generation_fenced` passed 10/10
  alone but failed once in the parallel focused set because it asserted
  `snapshot_ready` before the outbound worker recorded delivery. It must use a bounded
  observation wait so the strict parallel gate is deterministic.

The review's focused results were Engine DJ 11/11, app DJ 3/3, and the corrected D3
unlock fixture 1/1; I/O DJ was 25/26 in parallel and 26/26 serial. Compiler warnings
were zero and targeted diff-check passed. D3 remains blocked until these two P1s and
the deterministic-test P2 are fixed and a new independent fixed-hash review returns
P0/P1 zero.

### 9.3 KDMX DJ transport corrective checkpoint pending final review

The two P1 findings and the deterministic-test P2 have now been implemented, but this
checkpoint is deliberately **not accepted** until the independent fixed-hash re-review
finishes. State Sync physicality is classified once from the validated payload at
admission and stored on the inflight reservation: only `released=false` with an
explicit `loopDivision` consumes the process fence/cap and is retained through
completion, rejection, and listener retirement. Start, absolute Loop, Beat Jump, and
Release now share a queued/admitted/committing/finished cancel-or-commit receipt;
pre-commit cancellation wins without B, while a commit winner must return the
definitive publication or exact-A rollback result rather than a caller-synthesized
timeout. The asynchronous snapshot-ready assertion now uses a bounded observation
wait.

Implementation-owner gates on the resulting snapshot:

- Engine DJ focused: 12 passed / 0 failed; full Engine: 818 passed / 0 failed /
  2 ignored.
- I/O DJ focused: 28 passed / 0 failed; full I/O: 145 passed / 0 failed / 1 ignored.
- App DJ focused: 3 passed / 0 failed; D3 unlock fixture: 1 passed / 0 failed; full
  app no-default: 935 passed / 0 failed / 5 ignored.
- `cargo check -p syndocal --locked --bin syndocal --no-default-features`, full
  workspace fmt check, and targeted diff-check passed. First-party compiler warnings
  were zero.

Supervisor SHA-256 remeasurement matched the implementation report:

- `crates/engine/src/lib.rs`:
  `635D18D97FE758D3DD2686B278DE200F7E747D2926289E016ECBB064E13A4EAA`
- `crates/io/src/remote_ws.rs`:
  `1D1588163465FBFEBD41BB98942E8CEC860BE8F9480FA87943261460C4C21264`
- `app/src-tauri/src/main.rs`:
  `2DF9675D78E405D49B4A9EAD7F37DB59BFA3C26D068B4A114DA0E5C2F8393AFC`

The independent re-review verified that the State Sync admission-time classification,
stored physical bit, four quadrants, and both socket adapters close the earlier P1.
It nevertheless returned a new **FAIL, P0=0/P1=1** before D3 could be unlocked. In
`DjLinkCommandReceipt::wait_for_result`, a `Committing` receipt waits indefinitely
after the three-second caller window plus 750-ms margin, and I/O `stop_and_join`
also performs an unbounded worker join. The snapshot read-guard failure path is
bounded and rolls back, but there is no structural or test proof that every
post-commit path finishes before the listener's four-second stop contract. Simply
returning a timeout is not a valid repair because that would recreate a rejected ACK
followed by late B.

Supervisor start/end remeasurement confirmed that the three hashes above did not
move during this read-only review. The snapshot has been assigned back for a third
correction: keep cancellation before commit, reduce the commit-winning critical
region to structurally bounded nonblocking work, return only a definitive publication
or exact-A rollback receipt, and prove actual-socket stop/replacement in under four
seconds for all four Engine APIs and contention/disconnect cases. No native/default
build, hardware acceptance, commit, or KDMX push has been performed. D3 remains
blocked until a later fixed-hash review is green.

### 9.4 KDMX DJ bounded-commit corrective checkpoint pending final review

The third corrective implementation is now stable, but remains **not accepted**
until its independent fixed-hash review completes. The four DJ Engine APIs now apply
B only as unpublished, worker-local provisional state. Before the receipt can enter
`Committing`, the worker builds the complete canonical Engine snapshot and audio
commit/projection images, reserves the snapshot and audio publication guards in one
order with deadline-aware nonblocking acquisition, and rechecks cancellation. The
commit-winning region contains only prepared moves/swaps, Copy assignments, an
atomic generation store, and receipt completion; it contains no blocking lock, I/O,
callback, condition wait, or variable-size image construction. Cancellation or
resource failure rolls the provisional runtime back to exact A before the next tick.
The repair also restored a previously omitted Beat Jump `position_ms` field.

The successful Engine receipt now carries its prebuilt canonical snapshot, so the
production app handler no longer reacquires the shared Engine snapshot after ACK.
The remote observer uses a nonblocking Engine snapshot read with a process-lived
last-good cache rather than inventing a default. Remote start/stop/replacement is
serialized by a lifecycle mutex, while the old listener is removed from AppState and
joined outside the AppState mutex before the replacement binds. Coordinator/runtime
admission in this production stop call graph is also nonblocking and returns Busy on
contention. No worker is detached.

The implementation-owner production loopback opens a masked RFC6455 client against
the actual `/dj-link` route, holds a snapshot/audio publication fence, queues a
Release behind an earlier definitive Engine load, and then stops the production
listener. It proves caller cancellation after at least two seconds and strictly
before four seconds, exact A with no late B, same-port serial replacement, process-
fence rejection of the same event, and listener/socket/client/inflight worker drain.
The complete test took 3.25 seconds. During full verification this test exposed an
additional observer/snapshot writer-priority cycle; the nonblocking last-good path
above is the resulting root-cause repair.

Strict implementation-owner gates on the final snapshot:

- Engine DJ: 16 passed / 0 failed; full Engine: 822 passed / 0 failed / 2 ignored.
- I/O DJ: 28 passed / 0 failed; full I/O: 145 passed / 0 failed / 1 ignored.
- App DJ: 4 passed / 0 failed, including the production loopback; D3 unlock fixture:
  1 passed / 0 failed; full app no-default: 936 passed / 0 failed / 5 ignored.
- App no-default Cargo check, full workspace fmt check, and targeted diff-check
  passed. First-party compiler warnings were zero.

The first I/O rerun also exposed three existing timing weaknesses and a fourth
remaining immediate assertion. They were not waived as rerun flakes: test-only port
allocation now binds port zero directly and reads the exact server local address,
eliminating the Windows probe/drop/bind TOCTOU while production still rejects port
zero; all four snapshot-ready observations now use a shared one-second bounded poll.
After those changes the focused and full I/O gates above were restarted from the
beginning and passed.

Supervisor SHA-256 remeasurement matches the implementation owner:

- `crates/engine/src/lib.rs`:
  `0B8754583AB9A4C20A0DEDD86ED9731E952FC0DE652AD7C5F096B1F832C8C43A`
- `crates/io/src/remote_ws.rs`:
  `24C0FFD2FDF0A3CBB58F045CBEAFA74A53B354EF99504564D22115355BFA2AD8`
- `app/src-tauri/src/main.rs`:
  `CF5909444303D4187F76CDA451678D3405BACCE9F4B68986D8BBB44758B40D32`

The independent reviewer is now attacking this exact start/end-hash snapshot. No
native/default build, hardware acceptance, KDMX commit, or KDMX push has been
performed. Integrated D3 remains locked until the review returns P0/P1 zero.

### 9.5 KDMX DJ bounded-commit fixed-hash review failure

The independent read-only review of the three hashes recorded in section 9.4 is
complete and returned **FAIL, P0=0/P1=1/P2=1**. Start and end SHA-256 values matched,
so the implementation remained stable throughout the review. D3 is still locked.

The P1 is in the process-lived `RemoteSnapshotCache`, not in the newly bounded Engine
commit region. A generic Web Remote worker and the listener's DJ observer call the
same provider concurrently. A worker can clone pre-commit A, pause before updating
the cache, allow the observer to publish and cache committed B, and then overwrite B
with A. Independently, `last_good.try_lock()` returns immutable startup A when the
pointer mutex is contended. The observer accepts either fallback as current truth and
can therefore emit a false B-to-A transition followed by a duplicate A-to-B transition
within the same valid DJ socket generation. The transport's session/process fences do
not and should not infer Engine snapshot publication order.

The assigned repair removes stale/startup fallback entirely. Snapshot unavailability
must be represented explicitly: the DJ observer retains its prior observation and
sends nothing, while generic `getSnapshot` fails closed with a retryable error and
does not replace the Web Remote's last good display. The repair must be proved through
an actual socket under deterministic unavailable/contention sequencing, including no
A regression, no duplicate B, and preservation of the existing strictly-under-four-
second stop, same-port replacement, and process replay-fence assertions.

The P2 is the extreme audio revision boundary in DJ Start rollback. At revision
`u64::MAX - 1`, provisional B can consume `u64::MAX`; a publication failure then
attempts a second fallible install to restore A and can fail before the complete
transport image is restored. Although normal runtime cannot realistically exhaust
that counter, it violates the exact-A rejection contract and is therefore also
assigned for a direct prebuilt-A restore plus a deterministic boundary test. No
native/default build, hardware acceptance, KDMX commit, or KDMX push has been
performed at this checkpoint.

### 9.6 KDMX DJ unavailable-snapshot and direct-rollback repair checkpoint

The implementation owner completed both findings from section 9.5 and stopped
editing the three assigned files. `RemoteSnapshotCache` and its startup/last-good
fallback are removed. The process-fenced snapshot provider now represents temporary
unavailability as `None`: the DJ observer preserves its previous semantic state and
sends nothing, while generic Web Remote `getSnapshot` returns a retryable error with
no `type` or `snapshot` payload. Production render-plan, I/O-plan, and external-video
status providers were also moved to nonblocking reads so an accepted worker cannot
hold listener shutdown past its deadline. An actual-socket test proves committed B,
two unavailable observations, and resumed B without startup-A regression or a
duplicate B; the production socket test additionally holds the video-transport
mutex during a generic request and still proves cancellation at or after two seconds
and strictly before four seconds, same-port replacement, and process-fence replay
rejection.

DJ Start publication-failure rollback now restores its prebuilt complete A image by
direct assignment rather than a second fallible timeline install. A deterministic
test starts at audio revision `u64::MAX - 1`, reaches provisional B at
`u64::MAX`, forces publication failure, and verifies authored/runtime timeline and
bank state, automation values and origins, clock, audio authority/revision,
signatures, generation, and `last_error` are exact A with no B on the next tick.

Implementation-owner gates all exited zero with zero first-party warnings:

- `cargo fmt --all -- --check`;
- Engine DJ 16/16, then full Engine 822 passed / 2 ignored / 0 failed;
- I/O DJ 29/29, then full I/O 147 passed / 1 ignored / 0 failed;
- app DJ 4/4, the production actual-socket test 1/1, the D3 fixture 1/1,
  and the operator-unlock fixture 1/1;
- full app no-default 936 passed / 5 ignored / 0 failed;
- app no-default Cargo check and the targeted three-file diff-check.

The supervisor independently remeasured this fixed snapshot:

- `crates/engine/src/lib.rs`:
  `FD5C425F801999044DE3FC1A9117D84B864BF06E578A7D6FDC46B454C1B1EB83`
- `crates/io/src/remote_ws.rs`:
  `0A9C7D703B9A161B3E76E031953816633A8DE53EC1AF4321A196440205DE9B72`
- `app/src-tauri/src/main.rs`:
  `A5BB9871A4FB36FCB148C1C6B0E9E8523C10CC676EFF81E403D9AF54EA24B093`

An independent reviewer is now attacking exactly those start hashes. D3 remains
locked until that reviewer returns P0/P1 zero and the supervisor confirms unchanged
end hashes. No native/default build, hardware acceptance, KDMX commit, or KDMX push
has been performed at this checkpoint.

### 9.7 KDMX DJ fixed-hash review pass and D3 unlock

The independent read-only review of section 9.6 is complete and returned
**PASS, P0=0/P1=0/P2=0**. Its start and end SHA-256 values matched all three hashes
recorded there. The reviewer independently confirmed the unavailable-snapshot
observer hold, generic fail-closed response, nonblocking production-provider call
graph, direct complete-A Start rollback at the final audio successor, the common
four-API bounded receipt, physical State Sync classification, cap/latch/TTL/replay
fences, flat ACK compatibility, and listener stop/replacement ordering.

Reviewer reruns were Engine DJ 16/16, full I/O 147 passed / 1 ignored, app DJ 4/4,
and the individual actual-socket unavailable, State Sync/cap/rate, final-revision
rollback, and four-API cancel/commit tests. Formatter and targeted diff-check also
passed. Every successful Cargo gate reported zero first-party warnings; the sole
ignored I/O test is the declared physical-MIDI environment gate. A reviewer probe of
the default-feature app stopped in external FFmpeg discovery because that shell did
not have the supported `VCPKG_ROOT`/pkg-config environment, so it is not claimed as
a source failure or as default-feature proof. The supported native environment and
default gate remain assigned to the later alpha.8 boundary.

The DJ transport dependency now unlocks D3. D3 itself remains unaccepted until its
own strict backend sequence and separate fixed-hash adversarial review pass. No
native/default build, hardware acceptance, KDMX commit, or KDMX push has been
performed at this checkpoint.

### 9.8 Vendor warning-gate repair and fixed-hash review pass

The first strict D3 run reached `windows-default-all-targets` and correctly failed
before compilation because the untracked vendored Tauri crate still contained
upstream warning-suppression attributes, while its generated `target` tree contained
Cargo suppression-environment text. The exact generated directory contained 1,829
files / 511,983,086 bytes. A direct deletion request was rejected by the execution
safety policy, so the supervisor verified both resolved paths and moved only that
reproducible directory to the recoverable quarantine
`C:\Users\kouty\AppData\Local\Temp\syndocal-vendor-target-quarantine-20260823`.
The vendor source, provenance, and licenses remain in the workspace; no vendor
`target` directory remains.

The implementation owner removed all 29 textual `allow(` sites from the three
vendored source files and fixed the actual default-Clippy causes locally: cfg-specific
unused variables and mutation, private Win32 helper naming, side-effect-only
`Option::map`, and the private eight-argument resize hit test. It did not weaken the
checker, add Rust flags, change dependencies, erase types, or change public/runtime
behavior beyond the previously reviewed Context Weak/strong backport. Implementation
gates passed with suppression count zero, vendor all-features/all-targets Cargo check
and Clippy `-D warnings` at zero warnings, workspace all-targets check at zero
warnings, workspace formatter/diff checks, and the formal
`windows-default-all-targets` ratchet with 11/11 artifacts and current warning counts
total/first-party/third-party = 0/0/0. The supported environment was VS2022 MSVC
14.43 x64, FFmpeg 8.1.2 shared, and LLVM; an initial Git `link.exe` selection was
diagnosed as premature `%PATH%` expansion and the complete gate was restarted with
delayed `!PATH!` expansion and the MSVC linker first.

The independent read-only fixed-hash review returned **PASS, P0=0/P1=0** and
remeasured identical start/end SHA-256 values:

- `vendor/tauri-runtime-wry-2.6.0-syndocal/src/lib.rs`:
  `5A7B3B5371D5135EE86133C0B914D275DF29AF58EA1B2B8B2AF7EF309CD14283`
- `vendor/tauri-runtime-wry-2.6.0-syndocal/src/util.rs`:
  `CBC67EF911A93DB7247EB007D5FC987735E167760D4FE60BDB460680ED23644D`
- `vendor/tauri-runtime-wry-2.6.0-syndocal/src/undecorated_resizing.rs`:
  `982F60A84E2DC5CBCED1A0610CD3E13EB26BC27F89C45EC88C60D1EBCECAEFC4`

The reviewer independently reran vendor all-features/all-targets Clippy, workspace
formatter, complete diff-check, and the same formal 11/11 zero-warning ratchet. It
found two nonblocking Windows-scope P2 boundaries in the existing Context backport,
not in the suppression cleanup: non-Windows raw display handles require target-local
lifecycle proof before Linux/macOS distribution, and direct external users of two
public runtime-internal types would see the backport's Rust source-shape changes.
This workspace has no such direct consumer, and Windows raw display handles are
pointer-free. These remain part of the explicitly deferred non-Windows/upstream
replacement boundary and do not block the local Windows D3 rerun.

Because the warning gate failed during the first attempt, the supervisor is restarting
the entire D3 sequence from gate 1. D3 is not yet accepted. No native build, hardware
acceptance, KDMX commit, or KDMX push has been performed at this checkpoint.

### 9.9 Full-suite DJ production-stop race repair and fixed-hash review pass

The restarted D3 sequence reached the complete app no-default suite, where the
production DJ socket lifecycle test exposed a real full-suite scheduling race. The
test's old `load_started` signal ran before command enqueue, while its shared snapshot
reader fence could let the 44 Hz Engine tick block on the snapshot writer before the
project-load command was consumed. Under that ordering the load remained `Queued` and
its three-second caller deadline cancelled it; treating that timeout as a successful
publication prerequisite would have weakened the stop/FIFO proof.

The repair adds a nondefault Engine `test-support` feature, enabled only by Syndocal's
dev-dependency. Its narrow submission handle enqueues the normal
`LoadProjectSnapshotPublished` command and pauses the Engine only after admission,
checked load, and `PendingCommandAck` registration but before the same turn's real
publication. The app test then acquires the real snapshot/audio reader fence, releases
the support barrier, sends the actual `/dj-link` RELEASE, and requires bounded server
stop, definitive successful project-load ACK after fence release, unchanged runtime
truth, same-port replacement, and process-fence replay rejection. Both explicit
`wait_result` and `Drop` release the support barrier idempotently, so a future omitted
release or an assertion panic cannot strand the Engine worker. The normal/build Cargo
feature tree contains no `test-support` edge.

Implementation-owner gates passed with zero added first-party warnings: the focused
test once and then 10/10 consecutive repetitions, full app no-default 936 passed / 5
ignored / 0 failed, production no-default Cargo check, workspace formatter, and
targeted diff-check. The independent fixed-hash adversarial review returned
**PASS, P0=0/P1=0**; after a surgical hardening follow-up it confirmed the prior
test-support self-wait P2 was removed. Final reviewed SHA-256 values are:

- `app/src-tauri/src/main.rs`:
  `8F66DA8F6C83BF0D7AD1C5252F5FA19D64D76020DD823AD83FB50EAC7624978A`
- `crates/engine/src/lib.rs`:
  `5886BE2668141438DAA82703BBD1A52AE33C06276F28E32CC83CE1FAF524EB2C`
- `app/src-tauri/Cargo.toml`:
  `0AC8A76E55DDAF3109B0017AF01045BA0EFCACB155DB6E22D084901E235AEA3F`
- `crates/engine/Cargo.toml`:
  `43063B598EF0B1D5F996F20DAC20C89DE30CC72699DF975B352123BD9C2CEF25`

The remaining nonblocking P2 is an existing scope boundary: generic
`RemoteWsServer::stop_and_join` joins retained workers rather than forcibly terminating
an arbitrary infinite handler. The four-second evidence is therefore exact for the
current three-second cancellable DJ handler, not a universal forced-deadline primitive.
Because this full-suite gate failed before the repair, the supervisor must restart the
entire integrated D3 sequence from gate 1 again. D3, native build, hardware acceptance,
KDMX commit, and KDMX push remain unclaimed at this checkpoint.

### 9.10 Integrated D3, Tauri adapter, warning, and frozen frontend closure

The strict sequence was restarted from gate 1 after every source correction and is now
green on branch `codex/syndocal-v1.2` at base HEAD
`23f350c366ede2fdffcfbf3232e18112eada51ea`. D3's independent final fixed-hash
adversarial review returned **PASS, P0=0/P1=0/release-blocking P2=0**. Full Rust
evidence is Engine 822 passed / 2 ignored, I/O 148 passed / 1 ignored, and Syndocal
no-default 951 passed / 5 ignored, with zero failures and zero first-party warnings.
The current accepted source snapshot includes main
`2CC94D3E5307E6BBC026F1EC815ABC02BFF47E987D62526446AFD1059984E6C7`, Engine
`F2DCE4D4F6E4B9CA4E15DB722608461D9D0ADBD9948FB41D5796FD4B94E1B2F7`, and I/O
`E552FEA70D017BBAE40B534A6D854BE1C1B20AFFE111B9E733E0C19FA7A1E8B4`.

The native Tauri command adapter covers all 51 flat commands: 39 common mutations,
11 video commands, and one repair command. Six focused adapter tests pass. Raw invoke
bodies are rejected without parsing, absent/null optional payloads preserve Tauri's
wire behavior, and malformed payloads return the standard invalid-args failure rather
than entering a mutation. The final main hash above received its own independent
fixed-hash review with P0/P1/P2 zero. This closes the raw-Tauri admission bypass at
the software/source boundary; it is not native crash-stress evidence.

The hardened warning ratchet also received an independent fixed-hash review with
P0/P1/release-blocking P2 zero. Its final hashes are library
`55D2388C3EA244F3C58B207E651E56F33DB10B6E76BD404A2943497E384926AA` and test
`3391F2F149ADCED3B5641E48E326680686B8D243389EAFC7E2DCC3799AEA7E55`.
Self-tests and the frozen frontend warning configuration passed with total and
first-party warning counts 0/0. Overflow, missing Git state, invalid Cargo lint
configuration, worktree/index/untracked suppression changes, and scanner ambiguity
fail closed rather than being treated as an empty or clean result.

The frozen frontend/static chain passed production build and warning gates, command
routing 133 renderer mutations / 29 server-authoritative routes / 33 raw calls / 402
facade calls, 417 frontend invokes, and backend inventory 478 commands / 311 literal
calls / 133 transactional mutations. Project transaction, authority, E3 recovery, E4
publication, output ownership/control/runtime, and the five-viewport Scene Matrix
strip and full matrix gates passed. The Scene Matrix checker now visits the real
Live/Playback desk before asserting executor Bank labels, then restores Edit/Scene
Matrix; checker SHA-256 is
`9074FDCB36544B1E9F893B9C41D477EA7F30CAC05D5ECA21F4D0BE4010C0D2AC`.
The DVC DMX source contract was strengthened to 35 assertions and passed at
`8076355D3A4A1A0CFE4CC05B477E7CCF175904131C235AC0E9D8CF0B0B15940F`.
Both corrections received fixed-hash reviews with no release-blocking finding.
Localization passed 3556/3556, and both worktree and cached diff checks passed.

The companion DJ Agent is independently closed on its `Beta` branch at pushed commit
`6c4f4328a6866d9d48022bd8ee20a7887c9de851`: 54 tests, 16 Node syntax checks,
zero warnings, and packaged `dist/server.exe` SHA-256
`339ECF6E82EB463F55977F63A137CB0CB52886CD7E2874E87F5AD4724234377B`.
This is package/software evidence only; rekordbox plus the packaged Agent, physical
DJ Link/State Sync, MIDI/Pedal, and failure recovery remain unaccepted on the target
machine.

The accepted product denominator remains exactly **19/71 (26.8%)**. D3 is closed as
a software/source gate without claiming a new native or physical acceptance row.
The intended final source manifest is
`qa/artifacts/source-freeze/2026-08-24-alpha7-pre-alpha8-source-freeze.sha256`;
the source-freeze owner will capture it after these documents stabilize. Product
metadata remains `1.2.0-alpha.7`. The next action is the synchronized alpha.8 product
version update and `check:release`, followed by the exact-path native build, one
responsive maximized window, WER/100-reload stress, current-PC displays and hardware,
ASIO/fault matrices, DJ physical acceptance, and the integrated one-hour soak.

### 9.11 Step 6 alpha.8 product-metadata checkpoint

The current product metadata is synchronized at `1.2.0-alpha.8` across the exact
20 authoritative points. The 20/20 coordinate audit, `pnpm --dir app run
check:release`, the eight-assertion `check:tauri-build-wrapper`, and `cargo metadata
--locked --no-deps` all passed. Locked metadata reports all eight first-party
packages at alpha.8. The Windows frontend TypeScript/Vite warning configuration
rebuilt successfully with output-marker coverage 3/3 and
total/first-party/third-party warnings 0/0/0.

Only the product SemVer advanced; project, template, cache, control-plane, command,
API, ABI, and audio-asset schema versions are unchanged. Historical alpha.7 and
pre-alpha.8 source-freeze evidence remains immutable. No alpha.8 native build,
launch, WER/reload, display, physical hardware, ASIO/DJ, or soak gate is claimed,
and the accepted denominator remains 19/71 (26.8%). The next action is the
exact-checkout native no-bundle build followed by native and physical acceptance.

### 9.12 2026-08-24 alpha.8 native crash-closure checkpoint

This checkpoint supersedes the alpha.8 build/launch claims above. The branch is
still `codex/syndocal-v1.2` at base HEAD
`23f350c366ede2fdffcfbf3232e18112eada51ea`. Product metadata is
`1.2.0-alpha.8`, synchronized at all 20 authoritative points. The frozen frontend
chain completed 25/25; the final Scene Matrix checker is
`E0FA2A5E0FA27A97CCCDCAE842BF6E6104270E85EBE8C1BE4FA572FE41915118`, and its
Gate 12 and Gate 13 each passed twice across all five viewports. The independent
Sol review returned P0=0/P1=0/release-blocking P2=0. The App initialization TDZ
repair hash is
`E62439B070BC210CDE4FE08DBDBAABBEF3E16C738A14F3925B3D9DB9AB60F241`; the native
reload harness hash is
`BBFDF9D65742964625DF127D581AA78E54F7B6638DCED67CCD08ADBFADBB2E42`.

The first alpha.8 native build attempt at 10:20 failed before source compilation
because Cargo selected `C:\Program Files\Git\usr\bin\link.exe` even though the
parent PATH displayed MSVC. No source failure was observed. The retry fixed the
selection with the absolute VS2022 14.43.34808 linker via
`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`. The required
`pnpm --dir app tauri build --no-bundle` then passed in 4m09s with zero
warning-shaped lines. The exact executable is 57,491,456 bytes,
SHA-256 `86423537C3F1DC8B3BBD602E7CC51F58F63DBC4CA05AEE2DAA3BCA485E8F6882`;
the PDB is 19,582,976 bytes,
SHA-256 `776DB6E8099DA6F996E2411CD81F3DDB1D3112EF09B9587582E9D352595A451B`.
Both FileVersion and ProductVersion are `1.2.0-alpha.8`.

Launch verification found exact PID `126188`, exactly one responsive `Syndocal`
window, maximized at `1920x1032`, with CDP on port 9333. The WER baseline was
recorded at `2026-08-24 10:27:02 JST`. The native reload stress artifact
`qa/artifacts/native-physical-acceptance/2026-08-24-alpha8-final/native-reload-stress-100.json`
has SHA-256
`99694A08414B52C8C35B567146E3E98006EF8BD2FF81CF1ED835F6B5697C004D` and passed
100/100 reloads with 100 unique time origins, exit 0, total 45.884s, min 319ms,
p50 439.1ms, p95 595.7ms, and max 717.3ms. Runtime exceptions and log issues
were both zero; the only console issues were the two expected Tauri reload
warning messages, each occurring 2,134 times. Post-run PID `126188` remained
responsive with one window; WER Event and Reliability deltas were both zero.
The observed process snapshot was WS 83,218,432 bytes, private 52,027,392 bytes,
538 handles, and 48 threads.

The current display topology has five active physical displays. D3, `MSI MPG321UX
OLED`, is the physical 3840x2160 4K panel, presented to the app at logical
2560x1440 with 150% scaling. The mandatory native route
`D5 -> D2 -> D3 -> D1 -> D6` is still pending; no 4K absence claim is valid.
Daslight PID `72476` (UDP 6454) and Ableton PID `103764` (UDP 20909) were left
untouched. DJ/rekordbox, physical MIDI/Pedal, ASIO, disconnect/fault matrices,
and the one-hour integrated soak remain pending or fail-closed where their
physical evidence is not yet available.

A read-only disk audit measured the workspace at 343.777 GiB, `target` at
343.04 GiB, `target/debug` at 238.15 GiB (149 GiB incremental), and the old
named QA target at 90.36 GiB. No cleanup has been performed; these are measured
size contributors, not a deletion authorization or proof that all data is stale.

Next action is to rerun the final frontend chain only if a later source/document
mutation requires it, record this checkpoint in a meaningful KDMX commit and push,
then perform the verified five-display native UI route. Do not mark full hardware,
ASIO, DJ, or release completion until their gates and the integrated soak pass.

### 9.13 2026-08-24 alpha.8 current-source build and reload checkpoint

This section supersedes the pre-CSS native binary and reload artifact named in
§9.12; it does not delete that historical evidence. The current branch remains
`codex/syndocal-v1.2` at pre-commit base HEAD
`23f350c366ede2fdffcfbf3232e18112eada51ea`, and the synchronized product version
is `1.2.0-alpha.8`. The final current-source frontend chain passed all 25/25
gates in `target/qa/alpha8-current-source-final-gates-20260824-114918`.
The final current-source CSS blob is
`4974a2f828b8b8bd1c9fbe43390d97d5d6702179`, the upper-workspace checker blob is
`73a43ecfb2c6f10c07fb638f84f50375de5213a0`, and the Scene Matrix checker SHA-256
is `F164CD5B5C6C130E1D27B21C6A04CB1C361CEE3346F08FA9DFF77DE522C5FE11`.
The independent fixed-hash review reported P0=0/P1=0/release-blocking P2=0;
the warning ratchet for the exercised configurations remained
total/first-party/third-party 0/0/0.

The authoritative staged-source inventory is
`qa/artifacts/source-freeze/2026-08-24-alpha8-current-source-freeze.sha256`.
It contains 103 payload records; the manifest envelope is excluded from its own
payload to avoid a recursive self-hash. The earlier
`2026-08-24-alpha7-pre-alpha8-source-freeze.sha256` is historical evidence only
and is not the source authority for this alpha.8 artifact.

After resolving the exact checkout executable path and stopping the prior
`target/release/syndocal.exe` process, the required command
`pnpm --dir app tauri build --no-bundle` passed from the Visual Studio Developer
Shell (MSVC 14.43.34808, Windows SDK 10.0.26100.0). The retry log is
`target/qa/alpha8-current-source-final-gates-20260824-114918/native-release-build-retry-vsdev.log`.
The resulting `target/release/syndocal.exe` is 57,491,456 bytes with SHA-256
`627BE88032774C7FA0A4C3CD3510A7BFB52E8ED0E76884ADD414B9BFD101F459`; its PDB is
19,582,976 bytes with SHA-256
`5548E4F4B2C3CBB38F1881AAA6C9299AE42211616A8A05E9189C3019838F56AB`. Both
FileVersion and ProductVersion read `1.2.0-alpha.8`. Launch verification found
exactly one responsive Syndocal window for the exact executable (PID 123952 at
capture); the app was maximized before normal UI inspection.

The fresh current-source reload artifact is
`qa/artifacts/native-physical-acceptance/2026-08-24-alpha8-current-source-final/native-reload-stress-100-current-source.json`,
SHA-256
`6D12AA14371B1C837DF67DDE80AB44CB1C6B1329F093567B81524E35076FB621`. It records
100/100 reloads, 100 unique origins, exit 0, total 40.1424 seconds, minimum
232.5 ms, p50 407.3 ms, p95 479 ms, and maximum 530.1 ms. Runtime exceptions,
log issues, and WER Application Error/Reliability deltas were zero. The two
known Tauri console warning categories were observed as expected and are not
first-party warning-ratchet failures. The prior
`qa/artifacts/native-physical-acceptance/2026-08-24-alpha8-final` directory is
kept as pre-CSS historical evidence; the `...alpha8-current-source-final`
directory is the current-source evidence location.

### 9.14 2026-08-24 physical five-display pane route checkpoint

The mandatory native pane route is complete on the current machine via the
supported `open_pane_window` placement path:
`D5 -> D2 -> D3 -> D1 -> D6`. Each pane was maximized and each route step
verified zero document scroll. The measured route evidence is:

- D5: monitor 1920x1080, scale 1.5, pane viewport 1280x650, position
  `{-2465,1731}`;
- D2: monitor 1920x1080, scale 1, pane viewport 1920x1009, position `{0,0}`;
- D3 (`MSI MPG321UX OLED`): real physical 3840x2160 4K, scale 1.5, pane
  viewport 2560x1370, position `{-3840,-429}`;
- D1: monitor 2560x1440, scale 1.25, pane viewport 2048x1082, position
  `{1920,-364}`;
- D6: monitor 2560x720, scale 1, pane viewport 2560x649, position
  `{1598,1080}`.

Current-source route screenshots and SHA-256 values are:

- `D5-timeline-pane-route.jpg` —
  `37EDB56EA3F81BA014C23A96A1C78F32EEBB68C6B3E4C3D7C6692FD30E2FD0E1`;
- `D2-timeline-pane-route.jpg` —
  `0A037F7E6359FBC22C1FC16F297D9A3860157544777764679FABD2C194C20CFD`;
- `D3-4K-timeline-pane-route.jpg` —
  `55E83F6AFC32A32DD9BD5AAB187DA659DC0221DB0EF82EAEAA65215327F591CD`;
- `D1-timeline-pane-route.jpg` —
  `C22918826714D16E0E02DB5190AFB1E2724CEB387943A0D3FA4FABCDE5DD645E`;
- `D6-timeline-pane-route.jpg` —
  `2336172DB637B9E4F9D4906B3E7D6D781408CD0A4B11D18229974FC92E820340`.

The canonical machine-readable route record is
`qa/artifacts/native-physical-acceptance/2026-08-24-alpha8-current-source-final/native-display-route-current-source.json`,
SHA-256
`41F1D6E2528E7439657F8879F753255221E25F2DF0474139B56C1570E2C32C41`.

The expanded D5 Tools evidence is
`D5-timeline-tools-expanded-fixed.jpg`, SHA-256
`B780CD4CCDA35CC8A8F1148A64B26C91065EB15E33D57A38FDCC04F8B0A3724D`. Native
metrics record popup client/scroll `345/345`, each nested surface `335/335`,
and the deepest 44px target inside the viewport/popup with hit and focus proof;
Escape returned focus correctly. The pane was then closed. Final verification
found one exact responsive PID 123952, one CDP page, and the maximized D5 main
viewport was 1280x672.

All five detached Timeline pane screenshots still show the non-secret status six
seconds after each pane opened:
`Window 'pane-timeline' has no current project transaction owner registration`.
At the same six-second point, `get_project_authority_bundle` from the pane succeeds
with epoch 0/revision 1, but that separate read does not prove transaction-owner
registration or make the persisted status stale. This evidence accepts placement,
maximize, and containment only; it does not accept transactional pane operation,
warning-clean pane startup, or completed owner registration. The main D5 Timeline
Tools proof remains clean. Keep the owner-registration ordering/status behavior as
an alpha.9 P1 investigation boundary; it is not promoted to a completed acceptance
row.

This accepts the native multi-display pane placement/containment route only. It
does not claim display-output playback, fullscreen playback, GPU reset/recovery,
physical DJ Link/rekordbox, Stream Deck Pedal/MIDI, ASIO device operation, real
DMX output, disconnect/fault matrices, or the integrated one-hour soak. Daslight
PID 72476/UDP 6454 and Ableton PID 103764/UDP 20909 remain untouched. The disk
audit measured 343.777 GiB for the workspace, 343.04 GiB for `target`, 238.15 GiB
for `target/debug` including about 149 GiB incremental, and 90.36 GiB for the
old named QA target directories. The `.claude/worktrees` copies are only about
5.3 MiB each and `vendor` is about 0.29 MiB, so the size is overwhelmingly
generated Rust build and QA cache output, not historical source/worktree copies.
No cleanup was performed or authorized because the deletion/rebuild-cost tradeoff
was not authorized; generated `target` remains excluded from Git. The accepted
denominator remains exactly **19/71 (26.8%)**.

### 9.15 2026-08-24 alpha.8 pushed checkpoint and alpha.9 start

The independently reviewed alpha.8 checkpoint was committed as
`ec9fca4887e079fa61950056d94aca5ab5d65da9` (`checkpoint: validate Syndocal
1.2.0-alpha.8`) and pushed successfully to `origin/codex/syndocal-v1.2`.
Post-push verification was HEAD=upstream with ahead/behind 0/0, no staged or
unstaged tracked paths, and the same 19 intentionally excluded historical
alpha.7/pre-fix-alpha.8 evidence paths.

The active development train has now advanced to `1.2.0-alpha.9` at the exact 20
authoritative version coordinates. `pnpm --dir app run check:release` and
`cargo metadata --locked --no-deps --format-version 1` pass. Alpha.9 owns the
detached-pane registration-order P1: no completion claim is made until its focused
gates, required native build, fresh pane proof, independent review, commit, and
push all pass.

### 9.16 2026-08-24 alpha.9 owner-registration acceptance

Alpha.9 closes the detached-WebView transaction-owner registration-order P1.
Every renderer now has one sticky, fail-closed registration barrier before
owner-bound work; later trusted pointer/key or single-instance activity may
re-arm a cached failure without a retry loop. Only the main WebView consumes
startup or queued project opens, late work from a disposed WebView is discarded,
and selection/audio updates serialize to the latest desired value. The focused
contract fixtures cover concurrency, retry, disposal, and bootstrap ordering.

The checkpoint gates all passed: release metadata and locked Cargo metadata;
the 26-assertion Tauri wrapper checker; 417 frontend invoke assertions; 133
renderer / 29 server / 30 raw / 402 facade routing assertions; 478 backend /
308 operator / 133 renderer contract assertions; project transaction and
authority gates; E3, E4, output-ownership, output-control, video-window, and
timeline-follow runtime gates; 27 workspace/operator assertions; 3556/3556
localization assertions with zero bare strings; Cargo format; and ten focused
Rust tests. The frontend warning ratchet remained 0 first-party and 0
third-party warnings.

The required fixed native build
`pnpm --dir app tauri build --no-bundle` passed in 2m27s using the verified
MSVC 14.43.34808 Hostx64/x64 linker. The 57,489,408-byte EXE SHA-256 is
`BD4375D09EA09E099E6F24D74DC57B014E60F0C4C124401D1CBA6ACB1EE207FF`;
the 19,582,976-byte PDB SHA-256 is
`0DAF81F154F6C3953D5B9DC4A9DC85FF9889CC8CA5A277F4D635E80C2A17FC56`.
Fresh native acceptance at
`qa/artifacts/native-owner-registration/2026-08-24-alpha9/` opened all seven
real Workspaces panes, proved registration before initial owner-bound work,
proved pane ownership survived a main reload, observed no owner-status/error/
crash, and returned to exactly one responsive maximized main window. Tauri
enumerated all five connected displays including 3840x2160; this is enumeration,
not playback acceptance.

The accepted whole-product denominator remains **19/71 (26.8%)**. DJ Link/
rekordbox, DMX, MIDI/Pedal, ASIO, output playback, fault matrices, and the
integrated soak remain unaccepted. The newly observed blank lower band when both
Stage and Timeline are detached is an alpha.10 P1: the main layout must collapse
the unused band and expand the remaining content while keeping usable rejoin
controls and saved split ratios. The user has now authorized ongoing deletion of
verified-regenerable stale build/cache output; the alpha.9 commit/push is the
safety boundary before that cleanup.

The preceding collapse/remaining-content formulation is a historical alpha.10
observation and is **superseded by §9.20**'s real Timeline/Stage/Groups/Sources
semantic contract; it is not acceptance proof.

### 9.17 2026-08-24 alpha.9 push, stale-target cleanup, and alpha.10 start

The accepted alpha.9 checkpoint was committed as
`e9209d6de60eb6307045426b5c1b6f8c595c957d`
(`checkpoint: validate Syndocal 1.2.0-alpha.9`) and pushed successfully to
`origin/codex/syndocal-v1.2`. Post-push HEAD equaled upstream with
ahead/behind 0/0 before alpha.10 edits began.

The user authorized recurring removal of old generated output. A fixed,
fail-closed cleanup harness now lives at
`tools/cleanup-stale-targets.ps1`. Its dry-run resolved exactly 26 absolute,
non-reparse candidates beneath this checkout: 24 old Cargo target roots, the
old `target/x86_64-pc-windows-msvc` release tree, and
`target/debug/incremental`. It explicitly protects `target/release`,
`target/qa`, `target/root-warning-review`, `target/vendor-wry-review`,
`target/asio-qa`, `target/warning-capture`, source, and QA artifacts. At the
alpha.9 execution recorded below, the harness required no
Cargo/rustc/link/lld-link/mspdbsrv process and HEAD=upstream. That run predates
the clean-worktree guard: post-review hardening now requires every future
`-Execute` run to additionally have `git status --porcelain=v1` succeed with an
empty result, so current Execute requires both HEAD=upstream and a clean
index/worktree before any deletion. A later Ox P2 was also closed: the harness
now repeats the exact HEAD/upstream/clean checkpoint immediately before every
individual removal, alongside the existing per-path build-process recheck, so a
state change between planning and deletion fails closed.
An Ox adversarial review found no scope P0/P1. The post-delete zero-candidate
dry-run exposed a StrictMode sum bug that the review had missed; this was fixed
and reverified as `PLAN-SUMMARY paths=0 bytes=0`.

The cleanup permanently removed 26 verified-regenerable paths totaling
260,604,124,772 measured bytes (about 242.7 GiB). After cleanup, `target`
measured 112,699,182,397 bytes (104.959 GiB) and C: free space measured
439,990,702,080 bytes (409.773 GiB). The protected alpha.9 release remained
`1.2.0-alpha.9` with unchanged SHA-256
`BD4375D09EA09E099E6F24D74DC57B014E60F0C4C124401D1CBA6ACB1EE207FF`,
and one exact responsive `Syndocal` process remained available. Deleted output
does not use the Recycle Bin and is recoverable only by rebuilding.

The working development train has advanced to `1.2.0-alpha.10` at the exact
20/20 product-version coordinates. Release metadata, locked Cargo metadata, and
the 65-group release self-test pass. Alpha.10 work is parallel: Ox owns the
dual-detached pane reflow and focused checker, independent Codex lanes own native
matrix/contract arbitration/display and DJ Link preflight, and Sol retains
integration. The completion denominator remains 19/71; no new hardware,
playback, ASIO, fault-matrix, or soak claim follows from cleanup or versioning.

### 9.18 2026-08-24 alpha.10 parallel checkpoint — historical, closed

The main checkout remains on `codex/syndocal-v1.2` at committed/pushed base
`e9209d6de60eb6307045426b5c1b6f8c595c957d`; alpha.10 changes are intentionally
uncommitted while focused review and native proof remain open. The permanent
delegation policy is now synchronized in `AGENTS.md` and the completion flow:
Sol owns decomposition/integration/completion claims, the capability order is
Sol > Ox-alpha (`opencode/x-preview-f-free`) > Terra > Luna, Ox is the default
bounded implementer/reviewer, Terra implementation requires independent Ox
review, and Luna Max is restricted to small explicit low-ambiguity units.
Eliminating avoidable elapsed time is an explicit obligation: every safely
independent available lane stays assigned, with serialization only for true
dependencies, exclusive native/UI work, destructive actions, or same-file
ownership.

The alpha.10 dual-detached-pane source now removes the entire lower workspace
band only when both Stage and Timeline are detached, lets the upper workspace
fill the released height, preserves Stage-only/Timeline-only behavior and saved
ratios, and keeps the Workspaces Stage/Timeline controls as the always-mounted
rejoin route. Focused results currently pass: TypeScript no-emit with zero
  diagnostics, 28 workspace/operator assertions, the pane-reflow checker at
1920x1080 and 1920x1032, and the pane-window checker at five viewports when run
alone. A concurrent browser run demonstrated that Chromium/CDP gates sharing
resources must be serialized; it is not a product failure. An independent Ox
ARIA review found one EDIT-mode dangling `aria-controls` P1 in the intermediate
diff. The integrated expression now points to the always-mounted LIVE inspector
  while collapsed and omits the EDIT target until it is mounted. The base split
  gate was already red before this tranche because it expected uppercase `Setup`
  from a lowercase data attribute and required the collapsed LIVE target that the
  base component omitted; alpha.10 corrects both the assertion and the ARIA
  contract rather than silently deleting the failures. TypeScript no-emit reports
  zero diagnostics, the Node/checker configurations report zero first-party
  warnings, and the independent Ox pane review found P0=0/P1=0 and approved
  integration subject to the native-only boundaries below.

Two non-overlapping backend investigations run in parallel. The D4 Stage audit
confirmed that nine frontend-ticketed mutations still lack one authoritative
backend transaction/receipt boundary, with reply-loss recovery, Engine error
propagation, and no-op history gaps still open. Its first Engine-only commit is
assigned to an Ox implementer in the clean companion worktree
`C:\Users\kouty\Documents\KDMX-d4-stage-transaction` on
`codex/d4-stage-transaction`, owning only `crates/engine/src/lib.rs`; Tauri and
frontend receipt work remain later serialized stages. ASIO persistence has a
five-stage plan; Stage 1 ProjectFile v1-to-v2 migration is assigned to an Ox
implementer in
the clean companion worktree `C:\Users\kouty\Documents\KDMX-asio-persistence`
on `codex/asio-persistence-v2`, owning only
`crates/protocol/src/lib.rs` and `app/src-tauri/src/main.rs`. It must receive a
separate Ox review before integration. The three old detached `.claude/worktrees`
(`cranky-wright-6a0bf2`, `dazzling-spence-06b8ac`, and
`eager-pasteur-c3e6f4`) passed exact-path deletion review: each was detached at
the already-reachable `dd3f70b`, had zero tracked/untracked changes, was not a
reparse point, and had no external process user. `git worktree remove --force`
then removed their two ignored local settings files and worktree copies, reclaiming
14,882,234 measured bytes, and `git branch -d` removed only the three confirmed-
merged matching local branches. The commits remain reachable from the current
history, but the ignored settings are not recoverable except from another copy.
No alpha.10 native build, native pane matrix, new hardware acceptance, commit, or
push is claimed at this in-progress checkpoint; the denominator remains
**19/71 (26.8%)**.

Correction of the earlier static claim in this section: the previously reported
alpha.10 "Tauri window-scoped emit" P1 does not exist. Against the pinned Tauri
2.5.1 source, `Emitter::emit` publishes to every event target even when called
on a `WebviewWindow`; only the private `emit_to_window` helper scopes built-in
window events to one target. The pre-alpha.10 single-line
`window.emit("syndocal://pane-window-closed", pane)` in the child `Destroyed`
callback therefore already reached the main renderer's placeholder-retirement
listener, and `main.rs` has been restored to that exact base form with no
behavior change. The two static workspace/operator assertions that required the
app-handle form and rejected the window-local form encoded the false claim and
were removed, restoring the gate to its accurate 28 assertions (27 base plus
the alpha.10 `data-workspace-pane-toggle` selector). Because a browser cannot
exercise real child windows, closing a detached pane with the native titlebar X
and observing main-window placeholder retirement remains an open native
acceptance test for this train; it is not proven by this correction. The
accurate focused browser evidence is unchanged: the pane-reflow gate passes at
exact 1920x1080 and 1920x1032 with the both-detached lower band measured at
zero height and the upper pane at 968/920 px respectively, explicitly proving
the zero-width context element stays present so the rectangle fallback cannot
vacuously accept a missing selector, and the pane-window gate passes at all
  five viewports when run alone. The fresh read-only Ox adversarial review
  independently reproduced every focused gate and approved the source with only
  P3 proof-strength/documentation follow-ups, now tightened to the deterministic
  13-cell inspector, 88/88/28px view controls, and 33/32px surface headers. The
  required release build,
native child-X proof, both entry/rejoin orders, restored/F11 sizes, and physical
3840x2160 at 150% remain open and therefore block the alpha.10 commit/push claim.

### 9.19 2026-08-24 alpha.10 pane semantic correction — historical, closed

The real native alpha.10 inspection invalidated the preceding dual-detached
layout interpretation before any commit or push. In Control/LIVE, the actual
Timeline is the upper `liveControlPanel`, Sources are the lower-right
`TimelineSourceShelf`, and the lower-left slot is intended to host the 2D Stage.
The current source instead overrides that Stage fallback with a sparse Timeline
Preview. Timeline detachment then hides Sources while leaving both the main
upper Timeline and the child Timeline visible; with the preview included, the
same concept can appear in three places. The integrated Stage is therefore not
failing to paint: its `MappingEditableStageShell` is not mounted in that mode.
An independent read-only Ox-alpha trace reproduced all of these findings. It
also proved that the previously green pane-reflow checker encoded the same
inverted contract by requiring Source width zero and no real Stage.

The corrected contract is now the alpha.10 blocker: integrated Control/LIVE
must show Timeline above and real Stage plus Sources below; Timeline detachment
must remove only the main Timeline and let Stage plus Sources use the full main
workspace; Stage detachment must remove Stage plus Groups and expand Sources;
both detached must leave Sources filling the main workspace. Timeline state
must not collapse unrelated Setup/Edit/Mixer content, and a persisted Timeline
expansion flag must not make the remaining band inert while its Timeline is in
a child window. The focused checker is being rewritten to identify the real
Stage, Timeline, and Source subtrees rather than accepting slot geometry alone.

Operational checkpoint: branch `codex/syndocal-v1.2`, committed/pushed base
`e9209d6de60eb6307045426b5c1b6f8c595c957d`; at that historical checkpoint,
alpha.10 work was still uncommitted.
The existing alpha.10 EXE is obsolete for this correction. Completion requires
the focused browser gates, independent adversarial review, a fresh native build
with the exact VS 14.44 Hostx64/x64 linker pinned instead of Git `link.exe`, and
real maximized/restored/F11/physical-4K child-window entry, reverse-order rejoin,
and titlebar-X proof. No new denominator item or native acceptance is claimed at
this in-progress checkpoint.

### 9.20 2026-08-24 alpha.10 pane semantic acceptance contract — accepted

The old alpha.10 dual-detached geometry/topology claim in section 9.18 is
**historical and superseded**: it described the upper slot as the surviving
workspace and did not require the real Stage and Sources. Section 9.19's native
trace remains the root-cause record, but its abbreviated layout wording is
superseded for acceptance by the exact contract below. Alpha.7 and alpha.8 are
immutable historical train evidence. The correction was subsequently committed
and pushed as `5c7e19a72a97e20f5ece553594841103990a78a9`; the final native/source
evidence and transient requested-pause state are recorded in
`qa/SYNDOCAL_POST_ALPHA10_PAUSE_HANDOFF_2026-08-24.md`.

The required Control/LIVE semantics are:

- **Integrated:** the real Timeline occupies the upper workspace; the real
  Stage and Groups occupy lower-left; Sources occupy lower-right.
- **Timeline detached:** only Timeline leaves the main window. The real
  Stage/Groups and Sources remain, together using the main workspace's full
  height.
- **Stage detached:** only Stage/Groups leave the main window. Timeline and
  Sources remain in the main workspace.
- **Both detached:** Sources are the only main-workspace content and fill the
  entire main workspace. No empty lower band or Timeline/Stage substitute is
  accepted.

Acceptance must exercise Timeline-then-Stage and Stage-then-Timeline detachment,
then each corresponding reverse rejoin order; it must also prove child titlebar
`X` closure, Timeline expand-before-detach and expand-before-rejoin behavior,
and correct placeholder retirement/rejoin. Every state must keep outer document
and app scroll at zero. Setup/Patch Groups must remain available, and Setup,
Edit, and Mixer must not regress. These are real-content requirements, not slot
or rectangle-only checks.

#### Final alpha.10 native measurement record

| Required native observation | Measurement/evidence | Status |
| --- | --- | --- |
| Maximized, restored, and F11 entry/rejoin at supported desktop classes | 1920x1032 / 1280x800 / 1920x1080; post-alpha.10 handoff section 2 | Pass |
| Timeline/Stage detach and reverse rejoin orders, including titlebar `X` | Both orders and reverse orders; unique Stage/Timeline children; post-alpha.10 handoff section 2 | Pass |
| Expand-before-detach and expand-before-rejoin | Preserved real Timeline state with no inert band; focused and native sequence | Pass |
| Integrated/one-detached/both-detached real-content topology and outer-scroll-zero | Alpha.10 evidence 03-07; document/app scroll 0 | Pass |
| Physical 3840x2160 display at 150% scaling | DISPLAY3, 3840x2088 work area, 2560x1392 CSS; evidence 12 | Pass on same committed source build |

Closure: the corrected source, focused gates, exact-linker native build,
maximized-window verification, independent adversarial review, and populated
measurement record were completed. This historical handoff remains superseded;
use the post-alpha.10 snapshot for executable hashes, cleanup, and residual
cross-checking, and use the active completion flow for current work.
