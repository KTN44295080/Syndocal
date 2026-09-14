# Luna handoff: implementation review before ShowClock continuation

## Current user request and stop boundary

The user accepted the SMC-Mixer LED test, requested autonomous `/goal` work
from the ShowClock handoff, and explicitly required review of all existing
implementation and necessary refactoring before continuing new implementation.
The latest instruction transfers continuation to **GPT-5.6 Luna**. The outgoing
agent stops after preserving this handoff; do not resume its Astra review agents.

Continue the existing task and goal, not a separate product roadmap or release.
Review scope is repository-wide, but the first pass below is partial. Do not
claim the entire codebase was reviewed or that the findings are already fixed.
Read `AGENTS.md`, the relevant completion-flow sections, and domain acceptance
before editing. Independent review is required for material runtime changes;
assign exclusive file ownership if delegating. Use Luna for this continuation;
the repository also explicitly prohibits delegated Terra work.

## Git, ownership, processes, and goal

- Workspace: `C:\Users\janua\OneDrive\ドキュメント\GitHub\Syndocal`.
- Active branch: `codex/showclock-review-20260912`.
- Implementation base: `747c27105754dc29aad8169716dce7fa8acfbf19`.
- Local `main` and live `origin/main` both resolved to that base at handoff.
- Before this document, the branch was clean and had no code or QA changes.
  No implementation agent wrote a file. This handoff is the only tracked
  change owned by this pause; its commit is identified by Git rather than a
  follow-up hash-only documentation commit.
- No task-owned additional worktrees or stashes were created; `git stash list`
  was empty. Existing unrelated worktrees below must be preserved.
- The three review agents (`review_showclock`, `review_runtime`,
  `review_frontend`) all ended with usage-limit errors. No agent remains
  editing. Protocol implementation was assigned but never started.
- Both baseline commands completed with exit 0. No Cargo, rustc, or Syndocal
  process was present in the handoff process inventory. No native app was
  built or launched by this review. Recheck processes before later work.
- Existing goal belongs to task `01a0943b-2bc9-75f2-92e5-fe398531e81a`,
  titled `SMC-MixerのMIDI Feedback確認`. Its last observed status was
  `usageLimited`, not complete. Do not replace or mark it complete merely
  because the model is changing; inspect live state on resume.

Unrelated worktrees under `C:\Users\janua\AppData\Local\SyndocalDev\worktrees`:
`macos-artifact-gate`, `macos-artifact-validation`, `macos-final-gate-20260908`,
`snapshot-profile-20260908`, `thumbnail-lifecycle-cancel-20260910`,
`thumbnail-recovery-20260908`, and `video-fx-browser-gate-20260910`.
Their changes were not audited or claimed by this task. Do not clean them.

## Preserved baseline evidence

These commands ran against unchanged implementation base `747c2710`:

| Command | Observed result |
| --- | --- |
| `cargo test -p protocol --locked` | 221 unit, 7 integration, 4 doctests passed; 0 failed/ignored; exit 0; no compiler warnings in captured output |
| `pnpm.cmd --dir app run check:release:static` | All constituent checks passed; exit 0 |
| Completion ledger, within static suite | 50 Open + 8 Deferred |
| Q1/Q4 ledger, within static suite | 32 Q1 rows, 29 domains, 12 decisions, 14 risks, 33 evidence; mirror parity passed |

Protected local evidence is under `target/qa/showclock-review-20260912/`.
It is ignored build-tree evidence, not a committed artifact. Preserve it;
cache cleanup is not part of this task.

| File | SHA-256 |
| --- | --- |
| `protocol-baseline.cmd` | `71ea9545544c20d6813ed80d523d899fad66a58a9d731bae7e017c11838002f6` |
| `protocol-baseline.log` | `2cb535382d4f759b52f1f60ef823b01f45c0b75593edd307f1fb92722cc7ee03` |
| `release-static-baseline.log` | `db22c333bd1e7fde6217c0da56fd9c5a05ce3eab403608d6d5f46ee9b7ce17c3` |

The direct Cargo batch initializes Build Tools `vcvars64.bat -vcvars_ver=14.44`,
pins `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER` to
`C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`,
and verifies that exact executable is first in `where.exe link.exe` before
Cargo. Follow `qa/WINDOWS_NATIVE_BUILD.md`; do not weaken the wrapper checks.
The predecessor reported unrelated full-tree `cargo fmt --check` differences;
this review did not rerun or repair that baseline. Format owned files only.
Docs-only handoff validation adds no compiler-warning measurement.

## Review findings: reproduce before changing acceptance

All line references below refer to base `747c2710`. These are source-audit
findings, not executed regression demonstrations. The existing passing checks
do not cover the described failure scenarios.

### 1. ShowClock protocol and admission

The independent reviewer read all 859 lines of
`crates/protocol/src/show_clock.rs`, all three tests, all call sites, and master
roadmap G1-G9/Q2. There are no production callers of this core yet.

- **P1: secret-bearing Debug.** `ShowClockPeerValidator` (308/316) and
  `ShowClockActionReceiver` (490/498) derive `Debug` while owning raw HMAC keys.
  Diagnostic formatting exposes the pairing key. Use a redacted secret owner
  or explicit redacted Debug; regression must prove recognizable secret bytes
  are absent from formatting for both owners.
- **P2: backwards sender monotonic time admitted.** `accept` checks sequence
  and show time (381-388), not `sender_monotonic_us` despite its documented
  ordering role. Higher sequence and show time with an earlier sender timestamp
  currently passes. Retain the last timestamp and reject atomically; prove a
  subsequent valid sample still passes and define equal-timestamp behavior.
- **P2: unbounded action history.** `accepted_actions` stores full canonical
  `Vec<u8>` values forever (500, 572-574). Define explicit session-lifetime
  bounded dedupe with compact fingerprints and fail-closed exhaustion. Never
  evict IDs in a way that enables repeat execution. Test duplicate/conflict
  handling at capacity, capacity failure, and unchanged admission state.
- **P2: clock-generation binding discarded on Re-arm.** The request's clock
  generation (582) is merely nonzero-checked and discarded (632-636). After
  clock 10/fence 2, Hold then clock 1/fence 3 arms successfully. Decide the
  coherent authoritative clock/fence contract before changing this API. A
  retained monotonic lower bound prevents rollback but does not itself prove
  the exact current clock generation; do not claim that stronger guarantee.
- Additional corrections: action sequence zero reports `ZeroGeneration`
  instead of `ZeroSequence` (425); derived identifier deserialization bypasses
  constructors; ASCII controls are not excluded by the current ID validator;
  enum declaration order determines canonical byte tags. Validate saved
  identity/configuration and use explicit stable tags and independent known
  HMAC/canonical vectors. Rename the test's exactly-once claim to at-most-once
  admission for one receiver instance.

Refactor into focused wire/canonical authentication, admission/dedupe, and
manual-fence modules under a thin `show_clock` export seam if practical.
Share validated peer context; avoid repeated validation/canonical allocations.
Preserve wire v1 unless a deliberate, tested version change is required.
No cryptographic dependency or public API change has been approved as a
specific design yet; select and review a concrete solution within this task.

Before any LAN integration, specify receiver reconstruction/restart behavior:
reusing session/key/generations currently resets replay history and accepts old
authenticated traffic. Nonce is only nonzero-checked. Require fresh paired
incarnation/session or authoritative restored replay state. Expiry is currently
shape validation only, not packet-age admission. Exactly-once execution and
crash persistence remain separate unimplemented boundaries.

### 2. Engine clock continuity

The runtime reviewer identified, and the root inspected, `BpmClock` in
`crates/engine/src/lib.rs` around 66115-66202:

- **P1:** `midi_clock_pulse` assigns `anchor = now` on every usable pulse
  (66139), while `snapshot` derives both fractional phase and absolute beat
  count only from time since that anchor (66178-66200). Continuous 24 PPQN
  MIDI resets phase every pulse and prevents beat count progression.
- `sync_external_clock` similarly reconstructs the anchor from only fractional
  phase (66160), losing integer beat progress on every sample. Verify caller
  semantics for wrap, seek, source changes and explicit transport resets.
- Consumers include Auto VJ (32595), queued global Beat/Bar clip takes (40924),
  and clock-synced FX/`RuntimeClipClockTracker`. The existing MIDI test around
  100412 checks BPM/lock but does not prove phase/count continuity.

First build deterministic synthetic-time 96-pulse and repeated phase-wrap
regressions, plus an affected queued-take regression. Separate tempo estimation
from continuous beat-position policy, preferably in an extracted clock module.
Do not introduce frame polling/allocations to solve this. Native acceptance is
required for integration, with physical MIDI timing remaining a separate gate.

### Engine clock continuity checkpoint

The engine clock finding is addressed in the current continuation checkpoint.
BpmClock now retains an absolute beat position while MIDI Clock tempo
estimation and external phase synchronization update their rate/phase; they
no longer reset the integer beat counter. The deterministic regression covers
96 MIDI Clock pulses, repeated phase wrapping, external re-sync, and the
existing queued Clip Take generation/hold boundary.

Evidence is recorded in
qa/ENGINE_CLOCK_CONTINUITY_REVALIDATION_2026-09-12.md. The focused clock
tests pass (5/5), the affected Clip Take test passes (1/1), and the full
engine suite passes (1070 passed, 14 ignored, 0 failed). This closes only the
current-source engine software slice. Estimator/bounded-slew/Hold/STALE,
LAN, native, physical, two-process, two-machine, and venue gates remain open.

### 3. DMX input timeout starvation

- **P1 source finding:** `crates/io/src/dmx_input.rs` 207-216 continues on
  malformed or wrong-universe packets before the signal-loss check at 244.
  After one valid frame, unrelated traffic faster than the socket timeout
  keeps the old universe marked present indefinitely. The engine clears its
  retained merge frame on `SignalLost`, not independent frame age.
- Reproduce on loopback only: timeout 120 ms, one U3 frame with channel 1=200,
  then U4 packets every 20 ms for over 500 ms. Require U3 SignalLost during
  traffic and merge clearing. Include malformed-packet traffic, no duplicate
  timeout events, and recovery on a valid U3 packet. No physical DMX is needed.

### DMX input timeout checkpoint

The DMX timeout finding is addressed in the current continuation checkpoint.
Malformed packets and valid packets for another universe are now classified
without bypassing the shared timeout check; only a valid frame for the
configured universe refreshes liveness. The new loopback regression reproduced
the pre-fix failure, then passed after the change while asserting SignalLost,
status clearing, no duplicate timeout, malformed-packet accounting, and valid
recovery. The DMX module and full IO suite also pass under the pinned MSVC
14.44.35207 x64 procedure. Evidence is recorded in
`qa/DMX_INPUT_TIMEOUT_REVALIDATION_2026-09-12.md`.

This closes only the current-source IO software slice. Physical DMX, external
Art-Net nodes, native acceptance, LAN, two-process, two-machine, and venue
gates remain open.

### MIDI/OSC frontend authority checkpoint

The frontend lifecycle finding is addressed in the current continuation
checkpoint. The App wiring now preserves the typed trusted-flush result from
the mapping authority bridge. Targeted MIDI and OSC Learn accept their own
exact successful mapping ACK and update the continuation token before
reconnecting; foreign replacements and untrusted flushes remain fail-closed.
Connect/Start/Feedback routes also stop before IPC when the mapping flush is
not trusted. Deferred authority tests and the frontend routing contract pass,
the frontend build passes, and the pinned Windows Tauri `--no-bundle` native
build plus process/window-handle smoke passes. Evidence is recorded in
`qa/FRONTEND_INPUT_AUTHORITY_REVALIDATION_2026-09-12.md`.

This closes only the current-source frontend/native-build software slice. The
physical MIDI/OSC matrix, external clients, controller movement, LED/clock/MTC
observation, latency, LAN, two-process, two-machine, and venue gates remain
open.

### ShowClock runtime/LAN checkpoint

The ShowClock continuation now has a transport-neutral deterministic runtime
in `crates/protocol/src/show_clock_runtime.rs` and a manually paired exact-peer
UDP adapter in `crates/io/src/show_clock_lan.rs`. The estimator establishes its
initial offset from the first authenticated sample, bounds later correction to
1,000 µs per sample, honors the lower of 750,000 µs and sample expiry for
STALE, publishes monotonic show time, and requires explicit Manual Hold plus
an operator-confirmed advanced fencing generation to re-arm. The action
scheduler is bounded to 256 entries and ±10 seconds, applies signed late
policy, rejects clock/fencing generation drift, and the output gate requires
exact project/lease/audio/recording/clock/fencing generations plus owner and
Locked state before queue consumption.

The LAN adapter uses explicit unicast peer addresses and protocol version 1;
there is no discovery or automatic failover. Wrong-source traffic cannot
extend the absolute receive deadline, and malformed/oversized paired packets
fail closed. Protocol focused tests pass 16/16; the full protocol suite passes
234 unit, 7 integration, and 4 doctests; LAN focused tests pass 3/3; and the
full IO suite passes 184 with 3 ignored and 0 failed under the pinned MSVC
14.44.35207 x64 linker. Evidence is recorded in
`qa/SHOWCLOCK_RUNTIME_LAN_REVALIDATION_2026-09-12.md`.

This closes only the current-source estimator/scheduler/output-gate and
loopback transport slice. Tauri IPC/UI wiring, native-window proof, physical
output, real wired two-machine partition/rejoin, crash/restart replay-state
restoration, witness/interlock, and venue acceptance remain open.

### ShowClock Tauri IPC/UI and two-process checkpoint

This checkpoint adds `app/src-tauri/src/show_clock_ipc.rs`, the typed
Tauri commands `get_show_clock_status`, `start_show_clock`, `stop_show_clock`,
`schedule_show_clock_action`, `hold_show_clock`, and `rearm_show_clock`, and a
Setup/IO `ShowClock LAN` panel. The process-owned worker has explicit
Primary/Standby roles, signed 250 ms samples/actions, exact-peer UDP pairing,
authentication-before-estimation, a fixed-capacity generation-bound action
queue, clean joined shutdown, stale transition after Primary loss, and a
process-local session-incarnation fence. The UI keeps only non-secret pairing
settings; session IDs and pairing keys are not persisted, so restart requires a
fresh paired session/key. Manual Hold stops the Primary sample loop and
disarms the gate; operator-confirmed Re-arm advances fencing generation and
clears old actions. The output status is always fenced (`output_armed=false`)
in this checkpoint.

Evidence passing on the current source: Tauri worker/action/fence focused tests
`3/3`; two-process integration test `2/2`; TypeScript; Vite production build;
frontend invoke/routing checks (`463` commands); native admission inventory
(`522` commands, `18` negative fixtures); output-control contract; the
specified MSVC release build; and exact executable process smoke. QA/ledger
updates are included in this checkpoint before commit and push.

This closes action IPC, Manual Hold/Re-arm UI, and generation-bound scheduler
wiring at the process boundary. It does not close output-gate arm and local
output-ownership authority wiring, physical output, real wired two-machine
partition/rejoin, replay restoration after crash/restart, witness/interlock, or
venue acceptance.

### 4. MIDI/OSC frontend authority and lifecycle

The frontend reviewer traced `app/src/createControlInputController.ts` against
the root-owned `App.tsx` mapping persistence bridge:

- **P1:** target Learn captures an exact epoch/revision/hash, changes mappings,
  awaits persistence, then compares the original identity (MIDI 291/335-337;
  OSC 644/687-689). `App.tsx` adopts the successful ACK's new revision/hash
  (14372) and retires mapping-driven inputs (14384-14388). Learn therefore
  rejects its own successful commit and skips intended reconnect.
- **P1:** Connect/Start captures device/config before flush but authority only
  afterward (MIDI Clock 158-173; MIDI Control 405-420; Feedback 443-456; OSC
  732-743). `App.tsx:17728` discards flush `{trusted, ownAcknowledgements}`.
  A pending project-A gesture can consequently start a project-B runtime after
  replacement. Capture initiating identity and preserve trusted own-ACK
  provenance; never rebase to unrelated project state.
- **Candidate requiring deferred-promise reproduction:** auto-feedback queue
  captures authority when dequeued, not when queued (503-521); old captured
  mappings may be rebound to a newer project. Invalidate/coalesce stale queued
  configuration and prove no stale command is dispatched after retirement.
- Normal mapping changes intentionally retire mapping-driven inputs/feedback.
  Do not remove this safety behavior to make Auto remain connected. The
  reviewer retracted that behavior alone as a bug.

Use a typed trusted flush/own-ACK bridge similar to existing media/publication
paths. Add real deferred-promise behavior tests for own ACK, foreign mutation,
project replacement, timeout restoration, and stale queue disposal. The app is
SolidJS, not React. Do not replace exact identity fences with epoch-only checks.

## Review coverage still open

The inventory counted 76 Rust files/249,606 lines under crates, 121 Rust
files/271,376 lines under the native app, and 342 TS/TSX files/131,739 lines in
the frontend, including tests. These counts are scope, not coverage evidence.
Only ShowClock's new module received full-file review. Runtime/frontend reviews
covered the seams above; the root inspected native standby/output ownership
entry points and related QA but did not complete their audit. Full native
backend, audio/video lifetime, security/persistence, remaining UI, parsers,
build/release and test-harness review remain to be inventoried and completed.
The tentative MTC/drop-frame observation was not verified and is not a finding.

## Completed SMC-Mixer LED slice

On 2026-09-12, the user explicitly requested a point-light test. Windows WinMM
enumerated exactly one output named `SMC-Mixer` at index 2. A Python ctypes
test opened only that output and sent channel-1 Note On messages for notes
0, 8, 16 and 24, value 127 for 2 seconds then value 0 for 1 second, three cycles.
The sequence ran 15:11:24-15:11:33 JST; every send succeeded, final OFF was sent
in cleanup, the port closed, and the process exited 0. The user confirmed the
visible response: `きました。大丈夫そうです`, then declared this slice complete.

This proves USB MIDI LED on/off on the connected unit. It is not a Syndocal
auto-feedback mapping/native UI, latency, MIDI Clock/MTC, or full input-matrix
acceptance. No firmware or saved controller configuration was changed. This
handoff preserves the new observation; `qa/M4_IO_VALIDATION.md` and the Q1/Q4
master mirror still contain the earlier send-only slice and should be updated
at the next applicable checkpoint without closing the full INPUT requirement.

## First safe Luna actions and subsequent order

1. Recheck Git/goal/process state, read this handoff and governing documents,
   preserve local evidence and unrelated worktrees, and continue on the current
   branch. Do not relaunch stopped Astra agents or manufacture goal completion.
2. Reproduce and fix the bounded ShowClock admission findings first. Assign
   exclusive ownership of `crates/protocol/src/show_clock.rs` and new child
   modules; keep manifest/lock/QA integration under one supervisor.
3. Independently review the stable diff, run focused regressions and full
   protocol tests with pinned MSVC, update one concise QA record and ledger
   mirror, inspect staged files, commit/push owned changes, and integrate main
   only when the applicable checkpoint is actually validated.
4. Address engine-clock, DMX-timeout and frontend authority findings in bounded
   tranches with the affected tests/native gate. Continue remaining repository
   review before claiming the user's review/refactoring phase complete.
5. Then continue ShowClock per master G9: connect the generation-bound action
   scheduler to local output ownership and a physical dispatcher, complete the
   native UI interaction matrix, and run the distinct two-machine fault/soak
   and physical-output gates. The software thresholds, pairing/key/restart
   policy, ports, action horizon/capacity/late behavior, and Manual Hold/Re-arm
   contract are now frozen and tested.
6. Two-process loopback is distinct from two-machine wired partition/rejoin
   and physical output. Follow Phase 5 audio/recording/live-source ownership
   prerequisites and do not infer hardware, venue, publication or total product
   acceptance from software checks. No additional DMX hardware output is
   authorized by this handoff alone.

Routine implementation, affected checks, independent review and owned
commit/push remain authorized; do not repeatedly ask for permission. Keep
architecture boundaries clear and record measured hot-path performance only
when actually measured. No product version bump, publication or cleanup is
needed for this documentation-only transfer.

## Continuation checkpoint — local output arm and dispatcher

This checkpoint supersedes the output-arm boundary described above for the
current source tree. It is based on branch
`codex/showclock-review-20260912` at `e8ad8212` and keeps the existing v1
transport decisions and additive compatibility behavior.

Implemented changes:

- `ShowClockActionPayload` is required for Release, Take, ClipLaunch/
  Transition, and TimelineJump; its kind, fields, IDs, and values are bound to
  the action before canonical HMAC bytes are accepted.
- `arm_show_clock_output` is a separate typed IPC command with mandatory
  operator confirmation. Primary arm acquires the existing local lighting
  ownership permit and arms the ShowClock gate. Standby arm requires LOCKED
  estimator state and the prior Manual Hold/Re-arm fence.
- Hold/Re-arm drop the permit and disarm the gate. Lighting actions are passed
  through the existing local `EngineHandle`; video Take/ClipLaunch/Transition
  reacquire local video ownership for each operation and fail closed in
  Standby. The Setup/IO panel exposes the explicit arm confirmation.
- The Tauri admission inventory and frontend routing classification include the
  new runtime mutation command.

Current evidence:

- protocol focused 17/17 and full protocol 235 unit + 7 integration + 4
  doctests, all passing;
- IO focused LAN 3/3 and full IO 184 unit + 3 ignored, with both two-process
  integration tests passing;
- Tauri ShowClock focused 5/5 and control-plane focused 30/30;
- frontend TypeScript/Vite build, 464 invoke commands, routing 133/31/28/471,
  native inventory 523 commands with 18 negative fixtures, and output-control
  checks passing;
- MSVC 14.44.35207 x64 release no-bundle build passed without first-party
  warnings. Exact release process smoke observed one responsive `Syndocal`
  window, issued maximize, and cleaned up only that exact executable path.
  The current executable SHA-256 is
  `75EE768A6500E3C0BCEFBF6B2F505046ECAFF82C8F57C42E30121266060D0CDE`.

The evidence proves the local software ownership/gate and dispatcher boundary,
not physical output. Native UI button-by-button observation, real wired
two-machine partition/rejoin/fault/soak, crash/restart replay restoration,
witness or physical interlock, automatic failover, venue, signing,
publication, and product-wide completion remain open. No additional physical
DMX output was emitted in this checkpoint.

Next safe action is to commit and push the owned source/QA/ledger mirror after
the applicable static checks. The remaining external gates must be executed
with the required native UI/device/two-machine topology and recorded against
the exact artifact; they must not be inferred from this local evidence.

## Continuation checkpoint — STALE safety closure

This checkpoint is based on the same branch at `4221ab42` and records the
follow-up safety changes after the local output-arm/dispatcher checkpoint.

Implemented changes:

- A ShowClock estimator that has entered `STALE` remains latched there when a
  later sample arrives. It can return to `Acquiring` only after explicit local
  Manual Hold and a newly advanced armed fence/re-arm sequence.
- The Tauri worker revokes the local lighting permit and disarms the output
  gate before polling queued actions whenever the clock state is `STALE` or
  `FAULT`, and publishes `auto_disarmed_unsafe_clock_state`.

Current evidence:

- protocol focused 18/18 and full protocol 236 unit + 7 integration + 4
  doctests, all passing;
- Tauri ShowClock focused 6/6, with the new STALE permit-revocation test
  passing;
- the prior IO focused/full and two-process loopback results remain unchanged;
- MSVC 14.44.35207 x64 release no-bundle build passed without first-party
  warnings. Exact release process smoke observed one responsive `Syndocal`
  window, issued maximize, and cleaned up only that exact executable path.
  The current executable SHA-256 is
  `916E8221F81984A8740DFF569CB988BBEA54A2ADBB633194C6E381988595E132`.

The updated ledger and roadmap mirror the current counts: ShowClock protocol
evidence 247 assertions, runtime/LAN evidence 21 assertions, and IPC/UI/two-
process evidence 46 assertions. This remains software and loopback evidence;
native UI interaction coverage, physical output, real wired two-machine
partition/rejoin/fault/soak, crash/restart replay restoration,
witness/interlock, automatic failover, venue, signing, publication, and
product-wide completion remain open and are not inferred here. No additional
physical DMX output was emitted.

Next safe action is to run the ledger validator and diff checks, then commit
and push this bounded safety checkpoint. Any remaining external gates require
their actual native UI/device/two-machine topology and separate evidence.

## Continuation checkpoint — atomic admission and canonical tag closure

This checkpoint is based on branch `codex/showclock-review-20260912` at
`4f793325` and records the next protocol hardening after the STALE safety
closure.

Implemented changes:

- `ShowClockPeerValidator` now separates validation from commit so the combined
  peer validator/estimator path validates both owners before either state is
  mutated. A generation rejection therefore cannot consume sequence state.
- Sample and action canonical bytes now use explicit stable wire-tag mappings
  instead of relying on enum declaration order. Existing canonical vectors
  remain unchanged.

Current evidence:

- protocol focused 19/19 and full protocol 237 unit + 7 integration + 4
  doctests, all passing;
- IO focused LAN 3/3 and the two-process integration test 2/2 passing;
- Tauri ShowClock focused 6/6, control-plane 30/30, frontend/static checks
  unchanged and previously passing;
- local rustfmt check passes for both changed protocol files;
- pinned MSVC 14.44.35207 x64 release no-bundle build passed in 3m35s with
  no first-party warnings. Exact release process smoke observed one responsive
  `Syndocal` window, requested maximize, and cleaned up only that exact path.
  The current executable SHA-256 is
  `DBD55E63DA4AA2CE629941838E1779081ED7A22107B11FEC4DD548F9C7CFEF3E`.

The updated ShowClock ledger and fenced roadmap mirror record protocol
evidence 248 assertions, runtime/LAN evidence 22 assertions, and the current
native hash. This remains current-source and loopback evidence; native UI
button-by-button interaction, physical output, real wired two-machine
partition/rejoin, crash/restart replay restoration, witness/interlock,
automatic failover, venue behavior, signing, publication, and product-wide
completion remain open and are not inferred here. No additional physical DMX
output was emitted.

Next safe action is to run the Q1/Q4 ledger validator and diff checks, then
commit and push this bounded protocol checkpoint. Any external gates still
require their actual UI/device/two-machine topology and separate evidence.

## Continuation checkpoint — current-source native artifact refresh

This checkpoint is based on branch `codex/showclock-review-20260912` at
`35d40f12` and refreshes the native artifact proof after formatting the
current protocol sources.

The exact pinned MSVC 14.44.35207 x64 `tauri build --no-bundle` completed in
3m32s without first-party warnings. The exact
`target/release/syndocal.exe` process smoke observed one responsive
`Syndocal` window, requested maximize, and cleaned up only that exact path.
The resulting current-source executable SHA-256 is
`ABE07C3592AC2F06EA94BD19E1CBCBE592848224430451AFD3FA712E954F34B5`.

The ledger and fenced roadmap mirror now point at this refreshed artifact.
This remains software, native process-smoke, and loopback evidence only;
native UI button-by-button interaction, physical MIDI/OSC/DMX/Art-Net output,
real wired two-machine partition/rejoin/crash/restart/soak, replay restoration,
witness/interlock, automatic failover, venue behavior, signing, publication,
and product-wide completion remain unaccepted external gates. No physical
DMX output was emitted in this checkpoint.

Next safe action is to run the ledger validator and diff checks, then commit
and push this documentation-only artifact refresh. External gates still
require their actual UI/device/two-machine topology and separate evidence.

## Continuation checkpoint — session-monotonic action admission

This checkpoint is based on branch `codex/showclock-review-20260912` at
`9c3a3672` and closes a concrete Primary-side replay path found during the
remaining IPC/output audit.

The generation-bound action scheduler now retains a session-local monotonic
sequence floor. It rejects replayed or reordered Primary action sequences
before queue mutation, rejects same-time/same-sequence collisions rather than
overwriting the queued action, and resets the floor only after an explicitly
advanced armed-fence rebind. The regression proves both queued-action
preservation and post-dispatch replay rejection.

Current focused evidence is protocol 20/20 and full protocol 238 unit + 7
integration + 4 doctests, with IO LAN 3/3, two-process 2/2, and Tauri
ShowClock 6/6 also passing. The exact pinned MSVC 14.44.35207 x64 no-bundle
build passed in 3m33s without first-party warnings. Exact-path process smoke
observed one responsive `Syndocal` window, requested maximize, and exited
cleanly. The current-source executable SHA-256 is
`A8BC08E37C6C1E0BAA16BE8D5A8B74AF34D558F7F6C304B75D3D3D1CB6C9324D`.

This closes the local session-sequence replay/overwrite path only. It does not
provide crash/restart durable replay restoration or exactly-once effects
across process failure, and it does not claim native UI interaction, physical
MIDI/OSC/DMX/Art-Net output, real wired two-machine partition/rejoin/soak,
witness/interlock, automatic failover, venue, signing, publication, or
product-wide completion. No physical DMX output was emitted.

Next safe action is to run the ledger validator and diff checks, then commit
and push this implementation/evidence checkpoint. External gates require
their actual UI/device/two-machine topology and separate evidence.

## Continuation checkpoint — UI sequence recovery

This checkpoint is based on branch `codex/showclock-review-20260912` at
`3570ee5a` and closes the corresponding UI recovery path after the backend
sequence admission hardening.

The ShowClock panel now reconciles its action-sequence input from the latest
backend `last_action_sequence` on status polling and command responses. A
panel remount or refresh therefore advances to the next safe sequence without
persisting the session id or pairing key. Unsafe/non-safe integer status values
are ignored rather than coerced.

TypeScript/Vite production build passes with 354 modules, frontend invoke
inventory passes with 464 commands, and command routing passes with
133/31/28/471 counts. The pinned MSVC 14.44.35207 x64 no-bundle build passes
in 2m31s without first-party warnings. Exact-path process smoke observed one
responsive `Syndocal` window, requested maximize, and exited cleanly. The
current-source executable SHA-256 is
`2E7F01BF5DD427E4544494B2303194D9A4ABA2E51EFCDAFD747ABAC1C4737C95`.

This remains software and native process-smoke evidence; no native
button-by-button UI session was observed because the available computer-use
surface exposed no native app. Physical MIDI/OSC/DMX/Art-Net output, real
wired two-machine partition/rejoin/crash/restart/soak, replay restoration,
witness/interlock, automatic failover, venue, signing, publication, and
product-wide completion remain unaccepted external gates.

Next safe action is to run the ledger validator and diff checks, then commit
and push this UI implementation/evidence checkpoint.

## Continuation checkpoint — duplicate action status recovery floor

This checkpoint is based on branch `codex/showclock-review-20260912` at
`4483e25a` and closes the remaining local status-regression path found during
the ShowClock IPC audit.

Standby now publishes authenticated action admission through one status helper.
The helper keeps `last_action_sequence` at the maximum observed value instead
of letting an old authenticated duplicate overwrite it with a smaller value.
The latest action id/status and accepted-action count still update normally.
The focused regression
`duplicate_action_status_cannot_lower_sequence_recovery_floor` proves that a
duplicate sequence 2 cannot lower an existing sequence-5 recovery floor.

The exact pinned MSVC 14.44.35207 x64 no-bundle build passed in 2m37s
(180.02s wall) without first-party warnings. The exact-path process smoke
observed PID 39892 with one `Syndocal` window, `Responding=True`, window handle
2951068, maximize requested, exit code 0, and zero exact-path processes after
cleanup. The current-source executable SHA-256 is
`56BD2EF7B828D1597ECFD3C4EC842248FDC41A668A16D5C298AA14310854FCC2`.

Current focused evidence is protocol 20/20 and full protocol 238 unit + 7
integration + 4 doctests, IO LAN 3/3, two-process 2/2, and Tauri ShowClock
7/7. TypeScript/Vite, frontend invoke inventory, and routing checks remain
passing at 354 modules, 464 commands, and 133/31/28/471. The whole-repository
Cargo format check still reports a pre-existing unrelated formatting delta;
the changed helper itself is formatted and `git diff --check` passes.

This closes the local duplicate-status recovery path only. It does not claim
native UI button-by-button interaction, physical MIDI/OSC/DMX/Art-Net output,
real wired two-machine partition/rejoin/crash/restart/soak, replay restoration,
witness/interlock, automatic failover, venue behavior, signing, publication,
or product-wide completion. No physical DMX output was emitted.

Next safe action is to run the ledger validator and diff checks, then commit
and push this implementation/evidence checkpoint. External gates require
their actual UI/device/two-machine topology and separate evidence.

## Continuation checkpoint — generation-reset status and child-process reap safety

This checkpoint is based on branch `codex/showclock-review-20260912` at
`9eb7d584` before the pending checkpoint commit and closes two concrete local
boundaries found during the remaining ShowClock IPC/process audit.

Manual Re-arm now resets every generation-scoped action status field, not only
the scheduler length. After a successful re-arm, accepted-action count,
scheduled-action count, last sequence, and last action id all start at zero for
the new generation. The existing lifecycle regression now asserts these
values, so stale old-generation status cannot be exposed as current state.

The two-process loopback harness now owns its spawned child through a Drop
guard. If a parent assertion or setup path exits early, the guard kills and
joins the still-running child; the success path still waits for normal child
completion. This is test-harness process safety only and does not extend the
software evidence to a real two-machine topology.

Current evidence is Tauri ShowClock 7/7, two-process 2/2, and the exact pinned
MSVC 14.44.35207 x64 no-bundle build. The release build completed in 2m31s
(174.5s wall) without first-party warnings. Exact-path process smoke observed
one responsive `Syndocal` window with HWND 11863936, requested maximize, then
closed cleanly with exit code 0 and zero exact-path processes remaining. The
current-source executable SHA-256 is
`963EFCAF7F53D9E9E14A0AC6D3B7CFA6819CA56E096F90D6309AD2CC1A356EF4`.

`rustfmt --check` passes for the changed two-process test and `git diff --check`
passes. The full-file rustfmt check for the ShowClock IPC source still reports
only the pre-existing unrelated `matches!` formatting delta at
`app/src-tauri/src/show_clock_ipc.rs:1210`; it was not reformatted into this
checkpoint. No physical DMX output was emitted.

This closes local generation-status and test-child-reaping boundaries only. It
does not claim native UI button-by-button interaction, physical
MIDI/OSC/DMX/Art-Net output, real wired two-machine partition/rejoin or
crash/restart/soak, replay restoration, witness/interlock, automatic failover,
venue behavior, signing, publication, or product-wide completion.

Next safe action is to run the ledger validator and diff checks, then commit
and push this bounded implementation/evidence checkpoint. External gates still
require their actual UI/device/two-machine topology and separate evidence.

## Current continuation state — 2026-09-14 takeover

The continuation is now owned by the current task on branch
`codex/showclock-review-20260912`. The latest pushed checkpoint is
The native window/pane checkpoint recorded below is the latest pushed
checkpoint for this continuation. Its commit hash is authoritative in Git; the
branch is `codex/showclock-review-20260912`.

The earlier product-code change in this continuation is in
`app/src/components/ControlBothPanel.tsx`: the DMX, all-output, and Video
blackout controls preserve their existing lease-bound callbacks and visible
operator labels while exposing stable action names and `aria-pressed` state.
The focused checker observes those three semantics in
`app/scripts/check-control-upper-workspaces-browser.mjs`.

The later venue-soak checkpoint also changed only the bounded QA harness and
wrapper: `crates/engine/examples/syndocal_soak.rs` now admits the canonical
Mapping fixture and clamps the software-soak video-opacity floor, while
`qa/run-soak.ps1` fails closed on a false or missing report and tolerates only
a host exit-code property loss after a truthful passed report. Product runtime
and authored sample behavior remain unchanged by that harness fix.

The native maximized/F11 gate was retried from the current source after the
host GPU query reported `2560x1600`. The app-owned maximized client still
measured `1280x752`; the gate failed closed before F11 or UI interaction with
exit code `1`. The user display registry reported `AppliedDPI=192` (200%), so
the logical client boundary still does not satisfy the required `1920x1000` /
`1920x1080` gate. This result is recorded in the H5 and accessibility
checkpoints; it does not provide native UI or accessibility evidence.

The native gate's static/self-test path was then run independently and passed
`81 checks / 0 failed`, including dimension contracts, pane expectations, CDP
loopback ownership, process-lineage rejection, and fail-closed negative cases.
This confirms the acceptance checker is healthy; it does not convert the
live-display size failure into native acceptance. The retained local
`chatgpt/macos-artifact-validation` branch was also re-audited: it contains
unique unmerged thumbnail-admission/cancellation commits relative to `main`,
so it remains preserved rather than being treated as an orphan.

Current-source verification passed for TypeScript, Vite build, localization,
renderer routing, shortcut/workspace contracts, the full static release chain,
and the pinned Windows MSVC `14.44.35207` no-bundle native build. The exact
release artifact process smoke also passed for
`target/release/syndocal.exe` (SHA-256
`5D2B479C39AB28CC10F2961EAB156E6EF8127E7571189BFF85D50871ACCDC66E`), with
one responsive `Syndocal` window and zero exact-path processes after clean
shutdown. The rendered browser rerun was subsequently repeated with the
installed user Chrome executable passed through `CHROME_PATH`. The Control
upper-workspaces gate passed at all four supported viewports (`3840x2160`,
`2560x1440`, `1920x1080`, `1280x720`), including Both/nested-control
reachability, first Escape focus return, and zero runtime/console/log/harness
errors or warnings at every viewport. This refreshes rendered browser evidence
only; the native 1920x1080 window gate and physical/output boundaries remain
separate.

The current-host physical MIDI slice was also rerun against this source with
the pinned release toolchain. `SMC-Mixer` input index `1` and output index `2`
were opened through production `midir`, and one safe `B0 7B 00` All Notes Off
message was sent; the ignored test returned `1 passed / 0 failed / 0 ignored`.
This refreshes only enumerate/open/safe-feedback evidence. The full physical
input marker remains open for controller movement, Clock/MTC, latency,
reconnect, OSC/Remote, and native UI routing.

The real native maximized/F11 gate was attempted with the isolated QA build and
failed closed before UI interaction: the last client was `1280x752`, while the
gate requires at least `1920x1000` and a `1920x1080` monitor. A live display
query confirmed the only attached display was `1280x800` with a `1280x752`
work area. The QA process and listeners were cleaned up. This is recorded in
`qa/CONTROL_BOTH_CHECKPOINT_2026-09-14.md`,
`qa/UI_H5_CONTROL_CURRENT_SOURCE_CHECKPOINT_2026-09-14.md`, and
`qa/ACCESSIBILITY_CURRENT_SOURCE_CHECKPOINT_2026-09-14.md`; it is not a native
interaction or accessibility pass.

The migration corpus was rerun after takeover against source `ddf66fc2` with
the exact MSVC `14.44.35207` linker. Release compilation completed in 10m12s;
all 11 selected migration tests passed with zero failures and zero ignored
tests. The run covered 224 truncations, 5 malformed byte/number cases, 3 depth
cases, and 128 semantic/idempotency cases. The source audit confirmed the
existing Windows deny-write/delete and Unix version-CAS/private-snapshot media
coherence paths. `MIGRATION-COMPATIBILITY-001` remains Open because the broad
fuzz matrix, `DEC-FILE-ID-001` product decision, and real upgrade/downgrade
machine rehearsals are not substituted by source tests.

The migration source-continuity audit then confirmed that
`git diff --name-only ddf66fc2..HEAD -- app crates` returned no paths. The
recorded migration corpus therefore still applies to the current product
source; no additional result was fabricated, and the marker remains Open for
the unresolved file-identity decision and real upgrade/downgrade rehearsal.

The H5 source-continuity audit then confirmed no `app`/`crates` paths changed
after `894c690c`, the blackout semantic implementation commit. The post-change
native build/process smoke and fresh Chromium browser gate remain valid. A
later current-source native window/pane acceptance run is recorded below;
`UI-H5-CONTROL-001` remains Open because its broader native Control,
accessibility, physical, and external boundaries are not closed.

The AI3 native-ingress record now also links the existing SMC-Mixer physical
MIDI slice into the AI control Q1 row. It is explicitly bounded to input/output
enumeration, open, and one safe All Notes Off feedback message; no controller
movement, Clock/MTC, reconnect, OSC/Remote, DMX, Art-Net, or venue evidence is
implied, so `AI3-NATIVE-INGRESS-001` remains Open.

The AI3 physical re-Arm source checks were then rerun: project
transaction/authority, E3 recovery, six transaction-recovery scenarios,
output-control/Standby Sync, output ownership, and safety-blackout all passed.
No physical output was opened, so hardware retirement/re-Arm ACK and venue
behavior remain unaccepted.

The AI8 external current-source suite was also rerun: Agent Bridge 11 groups,
bootstrap 4 groups, admission 539 commands with 18 rejected negative fixtures,
fake loopback adapter 15 groups, and strict JSON 130 assertions all passed.
No external client, clean-machine install, release endpoint, signing service,
or physical output was contacted, so `AI8-EXTERNAL-ACCEPTANCE-001` remains
Open.

The ASIO license/package source gate was then rerun: packaging boundary 169
assertions and ABI-v3 contract 22 assertions passed. The normal package still
rejects ASIO injection and `distribution_approved` remains false. No GPLv3
separation, Steinberg agreement, public ASIO artifact, or legal approval was
created; `ASIO-LICENSE-001` remains Open.

The observability/support current-source suite was also rerun against the
current `16d9f886` tree before the documentation checkpoint. Status, release
self-tests, bundled-library, Windows artifact self-tests, and strict JSON all
passed; the release-mode Rust focused run passed `diagnostic_` 37, `updater_`
3, and `project_replacement_is_redacted` 1 with zero failures/ignored tests
and zero first-party warnings. This remains software-only evidence: no live
update endpoint, signed publication, clean-machine drill, device, or support
operation was exercised. The marker remains Open.

The `REMOTE-SECURITY-001` current-source suite was then rerun against
`da43420f`: Agent Bridge 11 groups, bootstrap 4 groups, Tauri admission 539
commands with 18 rejected negative fixtures, fake loopback adapter 15 groups,
strict JSON 130 assertions, and output ownership all passed. Pinned MSVC
`14.44.35207` protocol tests passed `agent_authority` 10 and `control_plane`
57 with zero failures/ignored tests. No real LAN/TLS, adversarial client,
public network, or physical RDM/TOD path was exercised, so the marker remains
Open.

The authoritative ledger remains structurally valid at 58 markers:
`27 Complete`, `8 Deferred`, and `23 Open`. The Open IDs are
`AI3-NATIVE-INGRESS-001`, `AI3-PHYSICAL-REARM-001`,
`AI3-DURABLE-ACCEPTANCE-001`, `AI8-EXTERNAL-ACCEPTANCE-001`,
`UI-H5-CONTROL-001`, `REMOTE-SECURITY-001`, `MIGRATION-COMPATIBILITY-001`,
`OBSERVABILITY-SUPPORT-001`, `ACCESSIBILITY-NATIVE-001`,
`ASIO-LICENSE-001`, `ASIO-FORMAT-MATRIX-001`, `ASIO-FAULT-MATRIX-001`,
`ASIO-SOAK-001`, `ASIO-LATENCY-001`, `ASIO-PERSISTENCE-PACKAGE-001`,
`DMX-ARTNET-001`, `DMX-USB-RDM-001`, `INPUT-PHYSICAL-001`,
`DJ-LINK-HARDWARE-001`, `VIDEO-PHYSICAL-001`, `VENUE-SOAK-001`,
`SHOWCLOCK-VENUE-001`, and `COMPARE-PINNED-001`. The Q1/Q4 mirror reports
`58/58` Flow references and `85` linked evidence records. External markers
remain Open until their named hardware, client, two-machine, venue, signed
artifact, or other external evidence exists.

Branch cleanup was performed after a live worktree/ref audit. The local
orphan `chatgpt/windows-integration-20260908` at `40ff409f` was deleted: it
had no worktree, no remote counterpart, and its native-thumbnail and loopMIDI
changes are represented by later current-source commits. The
worktree-attached branches were preserved. The unmerged local
`chatgpt/macos-artifact-validation` at `f4aa5fff` was also preserved because it
still contains a unique native-admission fix. Its remote counterpart was
deleted because it pointed only to the loopMIDI commit already represented by
current `221ebf7f` and had no attached worktree. `git fetch origin --prune` and
`git remote prune origin --dry-run` then reported no stale remote refs.

## Latest current-source checkpoint — native window and pane lifecycle — 2026-09-14

This checkpoint is based on branch `codex/showclock-review-20260912` at pushed
base `bfb1be55`. The owned implementation change is
`app/scripts/check-native-window-acceptance.ps1`: the checker now establishes
Per-Monitor V2 before physical Win32 dimension reads, suppresses CDP task
acknowledgement leakage, accepts intentional empty pane arrays, excludes only
the exact `tauri-plugin-single-instance` helper class/title pair, applies the
decorated Stage/Timeline child minimum, retries verified maximize and
foreground operations within bounded time, reaps both isolated dev ports after
restart, and reconciles only stale Stage/Timeline records belonging to the
exact QA PID. Unknown windows, titles, PIDs, and foreground targets remain
fail-closed.

The formal run temporarily selected the primary display mode
`1920x1080@180Hz/32bpp` from `2560x1600@180Hz/32bpp` using a reversible Win32
wrapper and restored the original mode in `finally`. It used the exact MSVC
`14.44.35207` Build Tools linker and exited `0`. The machine-readable report
was `%TEMP%\syndocal-native-acceptance-20260914-012135\native-window-acceptance.json`.
It recorded monitor/work area `1920x1080`/`1920x1008`, main maximized client
`1920x1008`, F11 client `1920x1080`, and Esc-restored max `1920x1008`.

The same 19-screenshot run passed Stage-first and Timeline-first detach/reflow/
rejoin orders, detached-pane restoration after a real process restart, exact
child adoption after main-window reload, direct Stage-child close retirement and
reintegration, and final fully integrated Timeline rejoin. The report retains
two explicitly unverified native boundaries: proving retirement of a previously
absent record and retaining an unknown-presence record. No claim is made for
native Control button-by-button interaction, native accessibility, physical
MIDI/OSC/DMX/Art-Net/sACN/USB/RDM/video/display output, external clients,
two-machine operation, venue/soak, signing, publication, or product completion.

The corresponding QA updates are in `qa/NATIVE_WINDOW_ACCEPTANCE.md`,
`qa/UI_H5_CONTROL_CURRENT_SOURCE_CHECKPOINT_2026-09-14.md`,
`qa/CONTROL_BOTH_CHECKPOINT_2026-09-14.md`, and
`qa/ACCESSIBILITY_CURRENT_SOURCE_CHECKPOINT_2026-09-14.md`. The Q1/Q4 mirror
adds evidence `EV-UI-NATIVE-PANE-DPI-CURRENT-2026-09-14`, bringing linked
evidence to `85`; the authoritative Flow ledger remains `27 Complete`,
`8 Deferred`, and `23 Open` out of `58`.

## Latest current-source checkpoint — native Control surface — 2026-09-14

The native checker was continued from pushed base `92d3b838` on the same
physical `1920x1080@180Hz/32bpp`
primary display. Its safe Control probe passed the exact four-tab census
(`edit`, `mixer`, `both`, `live`), ARIA tab semantics, one visible panel per
mode, zero document/app scroll, Lighting view actions, the full Video surface
with safe Audio In disclosure and explicit empty-show recording guard, Both
cards/two monitors/Back-GO-Release/three semantic blackout states, and the
Timeline arranger plus lower-right Source shelf. It generated four Control
screenshots in addition to the pane evidence and wrote
`%TEMP%\syndocal-native-h5-20260914-053000\native-window-acceptance.json`.

The probe was intentionally limited to navigation, reachability, semantic
state, and safe structure. It dispatched no output, recording, Take, blackout,
Arm, Take Over, or device action. The Timeline live-status rail remains
intentionally hidden in the shared native shell while the arranger owns the
upper surface; the Source shelf is owned by the persistent lower-right context
pane. The checker now measures those ownership boundaries directly. Display
restoration to `2560x1600@180Hz/32bpp`, isolated-port release, and QA-process
cleanup were confirmed. `UI-H5-CONTROL-001` remains Open because native
button-by-button/dangerous workflow, accessibility, physical/external,
failure/recovery, venue, signing, and publication boundaries are unproven.

The Q1/Q4 mirror adds `EV-UI-H5-NATIVE-SURFACE-CURRENT-2026-09-14`, bringing
linked evidence to `86`; the authoritative Flow ledger remains `27 Complete`,
`8 Deferred`, and `23 Open` out of `58`.

## Later current-source checkpoints — 2026-09-14

The following source reruns were completed one at a time after the takeover.
Each result was recorded in its domain checkpoint and kept the corresponding
Flow marker Open when the required external boundary was absent.

- ASIO: `ASIO-LICENSE-001` reran the packaging boundary (`169 assertions`)
  and ABI-v3 contract (`22 assertions`). Normal distribution still rejects
  ASIO injection and `distribution_approved` remains false; no GPLv3-separated
  artifact, Steinberg agreement, legal approval, or public ASIO package exists.
  `ASIO-FORMAT-MATRIX-001` reran live-audio lifecycle, IPC-v1, and ABI-v3
  checks; explicit device/rate/channel/native-format/fixed-buffer admission
  remains enforced, but no real-driver matrix was opened. `ASIO-FAULT-MATRIX-001`
  reran the SDK-independent bridge suite (`31 passed`) and ABI-v3 (`22`), with
  no real-driver reset, XRUN, unplug/replug, or callback-gap run. `ASIO-SOAK-001`
  reran lifecycle checks only; no device or one-hour run was performed.
  `ASIO-LATENCY-001` reran the current lifecycle contract only; no physical
  input-to-pixel or matched TouchDesigner measurement was produced.
  `ASIO-PERSISTENCE-PACKAGE-001` reran lifecycle and packaging (`169`),
  preserving explicit selection and normal-package ASIO rejection; native
  telemetry, stale-device locking, and final package review remain open.
- Outputs: `DMX-ARTNET-001` reran DMX addressing, ownership, safety-blackout,
  and show-setup checks; no Art-Net/sACN node, fixture, reconnect, or sustained
  physical 44 Hz output was exercised. `DMX-USB-RDM-001` reran output-control,
  ownership, Standby Sync, and safety checks; host inventory exposed only a
  generic FTDI `USB Serial Port (COM5)`, not a named Enttec/DMXKing/RDM path.
- Inputs and control: `INPUT-PHYSICAL-001` reran DVC MIDI (`39` assertions),
  DVC DMX (`41`), and frontend routing (`133/31/28/479`); the SMC-Mixer safe
  physical slice remains limited to enumerate/open and channel-1 All Notes Off.
  `DJ-LINK-HARDWARE-001` reran both policy/runtime checks without starting a
  rekordbox peer, Agent, pedal, or remote client. No live HELLO/ACK/STATE_SYNC,
  reconnect, or hardware artifact was created.
- Video and venue: `VIDEO-PHYSICAL-001` reran routing, managed-window,
  observation, and bounded-polling contracts; no real display, HDMI, NDI/Spout,
  or frame-drop evidence exists. A current-host `ASUS 5M webcam` was then
  exercised through the canonical DirectShow profile (`nv12`, `1280x720`,
  `30/1`) with the pinned MSVC `14.44.35207` ignored capture-worker test:
  `1 passed / 0 failed / 0 ignored`, two clean worker start/stop cycles,
  expected RGBA dimensions, and a nontransparent frame. This is only a
  partial camera capture/restart slice; it does not prove display/HDMI,
  NDI/Spout, unplug/replug, reconnect, frame drops, one-hour, venue, or
  product completion. `VENUE-SOAK-001` fixed the
  two harness continuity defects, then passed the exact pinned-source mixed
  software soak: `1801/1801` nonblank frames, `0` dropped, tick p99 `510 us`,
  command queue p99 `92 us`, command-to-DMX p99 `94 us`, `3603` successful
  loopback sends, `0` failures/render errors, and `26.4 MB` peak working set.
  This is 60-second software loopback evidence, not the required one-hour
  maximum-condition venue/GPU/integrated A/V/physical-output acceptance.
- ShowClock and comparison: `SHOWCLOCK-VENUE-001` reran protocol (`21`), LAN
  (`3`), Tauri IPC (`7`), two-process (`2`), frontend-invokes (`480`), routing
  (`133/31/28/479`), admission (`539` with `18` negative rejections), and
  output-control/Standby Sync checks. No real switch, second PC, crash/restart
  replay restoration, stale-peer rejoin, device-loss, physical output, or
  zero-simultaneous-output rehearsal was performed. `COMPARE-PINNED-001` then
  reran `pnpm.cmd --dir app run check:status` successfully; there is still no
  approved pinned SynapseRack/Daslight build, license, reference hardware,
  synchronized content/capture, or operator measurement package.

The authoritative completion ledger remains `27 Complete`, `8 Deferred`, and
`23 Open` out of `58`. No Open marker was changed to Complete from source-only,
loopback, inventory, or partial physical evidence. The current Open list is:
`AI3-NATIVE-INGRESS-001`, `AI3-PHYSICAL-REARM-001`,
`AI3-DURABLE-ACCEPTANCE-001`, `AI8-EXTERNAL-ACCEPTANCE-001`,
`UI-H5-CONTROL-001`, `REMOTE-SECURITY-001`, `MIGRATION-COMPATIBILITY-001`,
`OBSERVABILITY-SUPPORT-001`, `ACCESSIBILITY-NATIVE-001`,
`ASIO-LICENSE-001`, `ASIO-FORMAT-MATRIX-001`, `ASIO-FAULT-MATRIX-001`,
`ASIO-SOAK-001`, `ASIO-LATENCY-001`, `ASIO-PERSISTENCE-PACKAGE-001`,
`DMX-ARTNET-001`, `DMX-USB-RDM-001`, `INPUT-PHYSICAL-001`,
`DJ-LINK-HARDWARE-001`, `VIDEO-PHYSICAL-001`, `VENUE-SOAK-001`,
`SHOWCLOCK-VENUE-001`, and `COMPARE-PINNED-001`.

## Safe resume procedure after this takeover

1. Continue the Open markers one at a time. For software-supported markers,
   implement and focus-test concrete gaps before updating the ledger. For
   external markers, preserve the current-source evidence and record the exact
   missing owner/device/client/environment rather than converting a static
   pass into acceptance.
2. If the native gate is rerun, preserve the `1920x1080` physical-display
   requirement and the checker fail-closed boundaries; do not lower dimensions.
3. Every completed checkpoint must update the applicable current-source or
   domain QA document, preserve the 58-row ledger/Q1-Q4 mirror invariants,
   commit only owned files, push, and verify upstream equality.

## Takeover continuation — branch cleanup and AI3 durable recheck — 2026-09-14

The supervising checkout remains `codex/showclock-review-20260912` at pushed
HEAD `3c75c9a0caf68cbaabc7188531d1ca21bc08f671`; the working tree is clean and
its upstream is equal. Four clean branches whose tips were already reachable
from the current/main history were removed from both local and origin refs,
after their exact attached worktrees were removed without force:

- `chatgpt/macos-final-gate-20260908` (`89dea805`)
- `chatgpt/thumbnail-native-reload-20260908` (`bc39f709`)
- `codex/thumbnail-lifecycle-cancel-20260910` (`f40aa2fb`)
- `codex/video-fx-browser-gate-20260910` (`d6f6d53a`)

The branches `chatgpt/core-integration-candidate-20260908` and
`chatgpt/macos-artifact-gate` were retained because their attached worktrees
contain uncommitted changes. `chatgpt/snapshot-profile-20260908` was retained
because it has a unique unmerged QA profiling commit. The local
`chatgpt/macos-artifact-validation` was retained because it has the unique
unmerged native-admission fix `f4aa5fff`; its already-redundant remote ref had
previously been removed. No other branch was deleted.

The AI3 durable current-source recheck is detailed in
`qa/AI3_DURABLE_ACCEPTANCE_CURRENT_SOURCE_CHECKPOINT_2026-09-14.md`. Agent
Bridge (11 groups), bootstrap (4), Tauri admission (539 commands / 18 negative
fixtures), E3 recovery, output-control/Standby Sync, and output ownership all
passed with exit code `0`. This did not open native dangerous-action dialogs or
touch output. `AI3-DURABLE-ACCEPTANCE-001` therefore remains `Open` for the
named native No/Close, reply-loss/crash-restart, physical ACK, and five-display
acceptance boundaries; the authoritative ledger remains `27 Complete`,
`8 Deferred`, `23 Open` (`58` total).

The next `AI3-PHYSICAL-REARM-001` source checkpoint then ran the current
replacement Rust tests under the exact Build Tools MSVC `14.44.35207` x64
linker. `project_replacement` passed 7 tests, and the focused input-retirement
and output-lease atomicity tests passed 1 each, all with zero failures and
zero ignored tests. The result confirms the software-side fencing/retirement
boundary but does not claim physical output retirement, hardware ACK, or
re-Arm; the marker remains Open. The detailed record is in
`qa/AI3_PHYSICAL_REARM_CURRENT_SOURCE_CHECKPOINT_2026-09-14.md`.

The following `AI3-NATIVE-INGRESS-001` current-source recheck ran against HEAD
`3d50fd7e`: frontend command routing (`133` renderer, `31`
server-authoritative, `28` raw, `479` facade dispatches), Tauri admission
inventory (`539` commands with `18` negative fixtures rejected and the recorded
SHA-256), output-control/Standby Sync, output ownership, and safety-blackout
runtime all passed. No native client, device, endpoint, or physical output was
opened. The marker remains Open; only the previously recorded bounded
SMC-Mixer safe MIDI slice is physical transport evidence.

The `AI8-EXTERNAL-ACCEPTANCE-001` bridge recheck then ran against HEAD
`e4ca306d`: Agent Bridge (11 groups), bootstrap (4 deferred lifecycle groups),
admission inventory (539 commands and 18 rejected negative fixtures), fake MCP
loopback (15 groups), and strict JSON (130 assertions) all passed. No real
external client, clean install, signing/publication endpoint, or physical
output was contacted. AI8 remains Open for its external release and hardware
boundaries.

The `REMOTE-SECURITY-001` expanded source recheck then passed at HEAD
`f078feed`: Agent Bridge (11 groups), bootstrap (4), admission inventory (539
commands / 18 rejected negative fixtures), fake MCP loopback (15 groups),
strict JSON (130 assertions), output ownership, AI5 sidecar transport/auth,
AI6 administration UI, and AI7 adversarial-contract checks. No real LAN/TLS,
external Remote/Touch client, public endpoint, RDM/TOD device, or adversarial
network harness was used; the marker remains Open.

## Takeover continuation — current ledger and branch state — 2026-09-14

The current supervising checkout is `codex/showclock-review-20260912` at
pushed HEAD `986fba3f`. The working tree is clean and
`HEAD...origin/codex/showclock-review-20260912` is `0 0`. Both authoritative
validators pass: `23 Open + 8 Deferred + 27 Complete` (`58` total), and the
Q1/Q4 mirror reports `58/58` Flow references, `15` decisions, `14` risks, and
`86` linked evidence records.

The four individually continued markers in this takeover are still Open for
their external boundaries, with current-source evidence recorded in their
dedicated documents:

| Marker | Current result | Unclosed boundary |
| --- | --- | --- |
| `AI3-NATIVE-INGRESS-001` | Routing/admission/ownership/safety PASS; bounded SMC-Mixer safe MIDI slice retained | Native OSC/DMX/Remote, Art-Net/fixture, feedback/clock, reconnect, latency, venue |
| `AI3-PHYSICAL-REARM-001` | Replacement/retirement Rust tests `9/9` PASS | Physical retirement, explicit re-Arm, hardware ACK, venue matrix |
| `AI3-DURABLE-ACCEPTANCE-001` | Bridge/recovery/ownership/admission source checks PASS | Native No/Close, reply-loss/restart, physical ACK, five-display |
| `AI8-EXTERNAL-ACCEPTANCE-001` | Bridge/bootstrap/admission/fake-loopback/strict parser PASS | Clean install, real clients, restart/update, security, signing/publication, hardware |

The remaining Open rows are unchanged and are not silently promoted: H5
native live/dangerous workflows; Remote/Security external LAN/TLS and fuzz;
Migration cross-version/upgrade matrix; Observability signed update and
support drills; native Accessibility; six ASIO license/driver/soak/latency/
package rows; Art-Net, USB/RDM, physical input, DJ-Link, video, venue,
ShowClock two-machine, and pinned comparison acceptance.

Branch cleanup is complete for the clearly unnecessary refs. The deleted
local and `origin` branches were `chatgpt/macos-final-gate-20260908`,
`chatgpt/thumbnail-native-reload-20260908`,
`codex/thumbnail-lifecycle-cancel-20260910`, and
`codex/video-fx-browser-gate-20260910`. The only remaining non-current local
branches are retained because they have dirty attached worktrees or unique
unmerged work: `chatgpt/core-integration-candidate-20260908`,
`chatgpt/macos-artifact-gate`, `chatgpt/snapshot-profile-20260908`, and local
`chatgpt/macos-artifact-validation`; the first three have matching origin
refs, while the last is local-only and its redundant origin ref is already
deleted. No additional branch is safe to delete without discarding another
worktree's changes or a unique QA/native fix.

The `UI-H5-CONTROL-001` browser fallback gate was rerun against HEAD
`70c7fb30` using the repository's explicit Chrome path because Browser plugin
routing is unavailable here. All four supported viewports passed the Lighting,
Timeline, focus-return, and CDP diagnostics checks with zero runtime, console,
log, or harness errors. This remains rendered-browser evidence only; native
button-by-button and physical-output acceptance stay Open.

The `OBSERVABILITY-SUPPORT-001` release/diagnostic recheck passed against HEAD
`8300943a`: release self-test metadata `137`, candidate extraction `43`,
materialization `4`, Windows artifact checks `144`, strict JSON `130`, status,
and bundled-library checks all passed. Pinned-MSVC release Rust tests passed
`diagnostic_` 37, `updater_` 3, and redacted project replacement 1, with zero
failures/ignored tests and zero first-party warnings. No endpoint, publication,
or device was used; signed update, clean-machine, support-drill, and release
acceptance remain Open.

The `ACCESSIBILITY-NATIVE-001` current-source recheck passed against HEAD
`ecc1799e`: localization `3844/3844`, zero bare user-data labels,
terminology, empty states, project-history and Timeline keyboard routing,
stage labels, TypeScript, and the Vite build (`358` modules). The large-chunk
notice remains an advisory and first-party warnings were zero. No native
screen-reader/High Contrast/scaling/IME/reduced-motion matrix was run, so the
marker remains Open.

The `ASIO-LICENSE-001` current HEAD recheck passed packaging (`169` assertions)
and ABI-v3 (`22` assertions) with `distribution_approved: false`. No public
ASIO artifact or legal decision was made; the GPLv3-versus-Steinberg route
remains an explicit external decision and the marker stays Open.

The `ASIO-FORMAT-MATRIX-001` source recheck at HEAD `8aba7087` passed
live-audio lifecycle, IPC v1 exact mapping, and ABI-v3 (`22` assertions). No
ASIO stream or real driver was opened; the required rate/buffer/channel matrix
remains external and the marker stays Open.

The `ASIO-FAULT-MATRIX-001` current-source recheck at HEAD `1fcb675d` passed
the SDK-independent bridge suite (`31` tests) and ABI-v3 (`22` assertions),
with zero failures/ignored tests. No real ASIO driver fault matrix was run;
occupy/reset/XRUN/unplug/recovery acceptance remains Open.

The `ASIO-SOAK-001` current-source preflight at HEAD `767f1318` passed the
live-audio fail-closed lifecycle, availability, persistence, presentation, and
ordering check. No ASIO/WASAPI device or long-duration stream was opened; the
one-hour physical soak and telemetry remain Open.

The `ASIO-LATENCY-001` current-source preflight at HEAD `5a612989` passed the
live-audio lifecycle/availability/persistence/presentation/order contract.
Typed telemetry remains source evidence only: no ASIO stream, marker,
TouchDesigner session, or input-to-pixel measurement was performed, so the
five-trial physical latency marker remains Open.

The `ASIO-PERSISTENCE-PACKAGE-001` source recheck at HEAD `e43f2148` passed
live-audio selection/persistence/lifecycle and ASIO packaging (`169`
assertions). No licensed/public artifact, native stale-device rehearsal, or
final package was produced; the marker remains Open.

The `DMX-ARTNET-001` current-source recheck at HEAD `de9eb1c0` passed DMX
addressing, output ownership, safety-blackout, and show-setup contracts. No
Art-Net/sACN node, fixture, reconnect, or sustained physical 44 Hz output was
used; the marker remains Open for the external lighting matrix.

The `DMX-USB-RDM-001` current-source recheck at HEAD `94cbe220` passed
output-control/Standby Sync, ownership, and safety-blackout contracts. Read-only
PnP showed only generic FTDI `USB Serial Port (COM5)`; no Enttec/DMXKing/RDM
interface was opened and no bytes or analyzer capture were produced. The
two-fixture USB/RDM matrix remains Open.

The `INPUT-PHYSICAL-001` source recheck at HEAD `e874a918` passed DVC MIDI
shortcuts (`39` assertions), DVC DMX shortcuts (`41`), and frontend routing
(`133/31/28/479` renderer/server/raw/facade dispatches). The prior SMC-Mixer
safe MIDI slice remains the only physical evidence; no new device traffic or
OSC/Remote/Clock/MTC/reconnect/latency matrix was run, so the marker remains
Open.

The `DJ-LINK-HARDWARE-001` source recheck at HEAD `d9908a8f` passed track
mapping policy and DJ Link frontend/runtime contracts. No rekordbox peer,
Agent, pedal, remote client, or MIDI traffic was started; HELLO/ACK,
STATE_SYNC, reconnect, restart, and the 0/12 hardware matrix remain Open.

The `VIDEO-PHYSICAL-001` current-source recheck at HEAD `e93c6a47` passed
video routing, managed output-window runtime, observation, and polling
contracts, including exact-Both recovery and fencing. No display/HDMI,
NDI/Spout receiver, or new camera capture was used; only the existing ASUS
webcam slice remains evidence and the physical marker stays Open.

The `SHOWCLOCK-VENUE-001` continuity recheck at HEAD `c1c1a6b0` confirms that
the authenticated ShowClock, generation/fence, exact-peer loopback, Tauri IPC,
and two-process software evidence remains current. No real switch, second
machine, power-loss/crash/restart replay, stale-peer rejoin, device-loss,
physical-output observation, or zero-simultaneous-output venue rehearsal was
performed, so the venue marker remains Open.

The `COMPARE-PINNED-001` continuity check at HEAD `b9c95b3d` retains the
passing status-model contract only. No pinned SynapseRack/Daslight build,
license/tier, reference hardware/content topology, synchronized comparison
capture, operator measurement, or physical-output evidence exists. The marker
remains Open until the V01-V17 and Daslight matrices are run with first-failure
and unmeasured-row retention.

The `VENUE-SOAK-001` continuity record at HEAD `c1c1a6b0` preserves the
current 60-second mixed-lighting software loopback result: 1801/1801
nonblank frames, zero dropped frames, and zero loopback DMX failures. It does
not claim the named reference-machine one-hour run, maximum-condition
GPU/resource/thermal logs, integrated A/V/lighting/output/recording path, or
physical venue acceptance; the marker remains Open.

The 2026-09-14 read-only Windows inventory found the existing `SMC-Mixer`
MIDI endpoints, FTDI `USB Serial Port (COM5)`, ASUS 5M/13M camera devices,
the Realtek ASIO component, Elgato virtual audio, and an active Wi-Fi adapter.
It did not identify Enttec/DMXKing/RDM hardware, an Art-Net/sACN node, an
external NDI/Spout receiver, a DJ-PC/rekordbox peer, a second test machine, or
the pinned comparator environment. No device was opened or driven by this
inventory; it is availability evidence only and does not promote any
external-acceptance marker.

At HEAD `d332b25e`, the aggregate `pnpm.cmd --dir app run
check:release:static` gate passed end to end. It covered the ledger/Q1-Q4
validators, AI0-AI7 source contracts, F1/F2 generation and output ownership,
frontend/admission inventories, project transaction/recovery/publication,
media and snapshot contracts, safety/output control, ASIO packaging/v3,
live-audio IPC, Timeline, video polling/routing/observation, and camera UI
contracts. This is current-source software evidence only; the command opened
no physical output or external client and does not close the remaining native,
hardware, venue, signed-release, or comparator boundaries.

The current native artifact/process recheck at source HEAD `f9ea7810` built
successfully with Build Tools MSVC `14.44.35207`; the exact
`target/release/syndocal.exe` was `66,230,272` bytes with SHA-256
`C95CD9F832FF2E12E25FC132C87E7E2EC44BE11A27348AA94542FE20EC45A940`. One
exact-path `Syndocal` process exposed one responsive window and was cleaned up
by exact path. The standard native checker then failed closed before UI
interaction because the current primary monitor is `2560x1600` and no
`1920x1080` mode was enumerated. No output/device action occurred; native
Control button workflows and accessibility remain unaccepted.

A supplemental native run on the available `2560x1600` display generated
`%TEMP%\\syndocal-native-acceptance-20260914-045044\\native-window-acceptance.json`.
It recorded `2560x1504` maximized, `2560x1600` F11, exact restore, safe H5
Control semantic-state checks, pane lifecycle, restart restoration, reload
adoption, direct-child close reintegration, and final reintegration. Every
observable assertion printed `PASS`, and the acceptance wrapper exited `0`
after reaping the owned Tauri dev subtree. The subtree printed its expected
`ELIFECYCLE` shutdown line during intentional cleanup, but that did not become
the wrapper result after the narrow checker fix. This remains supplemental and
does not replace the missing formal `1920x1080` gate. No output/device action
occurred.

## Continuation checkpoint — native checker status and branch audit — 2026-09-14

The only product-repository change in this continuation is
`app/scripts/check-native-window-acceptance.ps1`: the successful main path now
ends with an explicit `exit 0` after `finally` restores the environment. This
prevents the cleanup `taskkill.exe` status from leaking into a successful
checker result; all assertion and trust-boundary failures still throw and
remain non-zero.

Evidence on the current checkout:

- `node app/scripts/run-native-window-acceptance.mjs -SelfTest`: `81` checks,
  `0` failed, exit `0`.
- Available-display supplemental run with
  `-MinimumMaximizedClient 2400x1500 -ExpectedFullscreen 2560x1600`: report
  `%TEMP%\\syndocal-native-acceptance-20260914-045044\\native-window-acceptance.json`,
  all observable assertions `PASS`, wrapper exit `0`.
- Formal `1920x1080` acceptance remains fail-closed when the current primary
  display enumerates only `2560x1600`; no dimension requirement was weakened.

The branch cleanup audit removed these already-unneeded refs from both local
and `origin`: `chatgpt/macos-final-gate-20260908`,
`chatgpt/thumbnail-native-reload-20260908`,
`codex/thumbnail-lifecycle-cancel-20260910`, and
`codex/video-fx-browser-gate-20260910`. No stale remote refs remain. The
remaining branches are retained because they contain unmerged work or are
attached to dirty worktrees: `chatgpt/core-integration-candidate-20260908`,
`chatgpt/macos-artifact-gate`, `chatgpt/macos-artifact-validation`, and
`chatgpt/snapshot-profile-20260908`. They must not be deleted or their
worktrees removed without resolving ownership and preserving those changes.

The ledger remains `27 Complete / 8 Deferred / 23 Open` out of `58`. No marker
is promoted by the supplemental run: `UI-H5-CONTROL-001` still requires the
formal display gate plus accessibility and physical/external acceptance, and
the remaining Open rows retain their documented hardware, venue, client,
signed-release, or product-scope boundary.

## Current-source aggregate recheck — 2026-09-14

At current HEAD `dce7dc48`, `pnpm.cmd --dir app run check:release:static`
completed with exit `0`. It revalidated the completion and Q1/Q4 ledgers,
AI0-AI7 source contracts, F1/F2 input and output ownership, frontend/Tauri
admission and routing, project transaction/recovery/publication, media
thumbnail/asset/snapshot/agent-bridge paths, output safety and leases, ASIO
packaging and v3 contracts, live-audio IPC, Timeline cue/audio/loop/transport/
Follow/watermark paths, project bootstrap, video runtime/routing/observation,
and camera input contracts. The run performed no physical output, external
client, venue, signing, or publication action; the 23 Open marker boundaries
therefore remain unchanged.

## External-state inventory refresh — 2026-09-14

The read-only present-device scan was repeated after the current-source gate.
It found the existing `SMC-Mixer` and `CustomMIDI1` MIDI endpoints, FTDI
`USB Serial Port (COM5)`, ASUS camera devices, the Realtek ASIO component,
Elgato virtual audio, and the `Technics EAH-AZ100`/`Onyx` audio devices. The
only matching live processes were `loopMIDI` and `MidiSrv`; no rekordbox/DJ
Link peer, TouchDesigner, Daslight, SynapseRack, OBS, NDI/Spout receiver, or
DMX/RDM controller process was present. The device inventory still showed no
Enttec/DMXKing/RDM interface or Art-Net/sACN node. No device was opened, no
MIDI/OSC/DMX/audio output was sent, and no external client was controlled.
This refresh changes no marker status and leaves the same 23 external Open
rows awaiting their named hardware, client, venue, or product decision.

## Blocked audit — 2026-09-14

The same external-acceptance blocker has now persisted across the successive
current-source, native, release, inventory, and per-marker audit checkpoints.
The live audit at HEAD `7d2b189f` confirmed:

- both authoritative ledger validators pass with `27 Complete`, `8 Deferred`,
  and `23 Open` (`58` total; Q1/Q4 mirror `58/58` references);
- the working tree and upstream are equal and clean;
- the only matching live processes are `loopMIDI` and `MidiSrv`; no
  rekordbox/DJ Link peer, TouchDesigner, Daslight, SynapseRack, OBS, NDI/Spout
  receiver, or DMX/RDM controller is running;
- the present-device inventory still has no Enttec/DMXKing/RDM interface,
  Art-Net/sACN node, second machine, pinned comparator environment, or
  accessibility test environment needed by the named matrices.

The remaining 23 rows cannot be closed by another source scan, fake loopback,
simulated device, or guessed physical output. The required next state change is
external: provide the named operator-approved hardware/client/venue topology,
release-controlled signing/update environment, accessibility environment, or
the explicit product/licensing decision. On resumption, run each marker's
prescribed matrix with raw logs, identity, timestamps, first-failure retention,
and operator evidence; then update only the corresponding ledger rows and
commit/push the evidence. The goal is left blocked until that external state or
decision changes; no marker is promoted here.

## Resumption checkpoint — formal native display gate after secondary display — 2026-09-14

The user added a secondary display, so the previously missing native-display
condition was rechecked. Live DisplayConfig inventory identified `DISPLAY2`
as `1920x1200` at effective DPI `96`; its enumerated modes include
`1920x1080`. A reversible wrapper saved `1920x1200@165Hz`, changed only
DISPLAY2 to `1920x1080@60Hz`, moved the isolated QA window there, and restored
the saved mode with Win32 result `0` after the run.

The formal native checker completed with wrapper exit `0` and report
`%TEMP%\\syndocal-native-acceptance-20260914-082019\\native-window-acceptance.json`.
The report proves monitor/work area `1920x1080`/`1920x1032`, maximized client
`1920x1032`, F11 `1920x1080`, exact Esc restore, safe Control semantic-state
coverage, both Stage/Timeline detach orders, restart record restoration,
main-window reload adoption, direct Stage-child-close reintegration, and final
full reintegration. All observable assertions passed. The deliberate owned
Tauri subtree termination logged `ELIFECYCLE`, but it no longer leaked into
the wrapper result after the checker fix.

This is a real native display/pane-lifecycle advancement, not whole H5 or
external acceptance. No output, recording, Take, blackout, Arm, Take Over,
physical device, external client, accessibility environment, or venue action
was performed. `UI-H5-CONTROL-001`, `ACCESSIBILITY-NATIVE-001`, and the
other external Open rows remain unchanged until their complete matrices are
run.

## Resumption checkpoint — current-source aggregate after external-boundary audit — 2026-09-14

At current source HEAD `467b25e1`,
`pnpm.cmd --dir app run check:release:static` completed with exit `0`. It
revalidated both authoritative ledgers, AI0-AI7, F1/F2, frontend/Tauri
admission and routing, project transaction/recovery/publication/bootstrap,
media/snapshot paths, output ownership/control/blackout, ASIO packaging/v3 and
live-audio IPC, Timeline cue/audio/loop/transport/Follow/watermark paths,
video polling/routing/window observation, and camera contracts. The focused
H5 browser gate also passed all four viewports with the explicit installed
Chrome executable and zero runtime/console/log/harness errors.

This aggregate is current-source and rendered-browser evidence only. It did
not open physical output, external clients, real LAN/TLS, ASIO/WASAPI streams,
DMX/RDM hardware, TouchDesigner, a screen reader, a clean machine, a signed
update endpoint, or a venue/comparator setup. No Open marker was promoted.
The ledger remains `27 Complete / 8 Deferred / 23 Open` (`58` total), and the
working tree/upstream are clean and equal.

## Resumption checkpoint — ASIO current-source contract tranche — 2026-09-14

After the secondary-display continuation, the following three ASIO Open rows
were audited one by one on the current branch. None was promoted because the
required external evidence is absent; all checkpoint commits were pushed and
the working tree/upstream are equal and clean.

- `ASIO-SOAK-001`: at `0d216dc2`, `check:live-audio` passed. The exact M5
  `run-soak.ps1 -PreflightOnly` invocation failed closed before Cargo because
  the ordinary PowerShell host lacked `VCToolsInstallDir` and the configured
  exact `vcvars64.bat -vcvars_ver=14.44` initialization was unavailable. No
  stream or report was created, so this is not soak evidence.
- `ASIO-LATENCY-001`: at `fd82d5cd`, `check:live-audio`,
  `check:live-audio-ipc-v1`, and `check:asio-v3-contract` passed (the ASIO v3
  checker reported 22 assertions). No physical stream, TouchDesigner marker,
  pixel measurement, or raw timing log was produced.
- `ASIO-PERSISTENCE-PACKAGE-001`: at `0cbc0e58`, live-audio, IPC v1, and
  `check:asio-packaging` passed (169 assertions). The normal-package rejection
  boundary remains intact. The separate Windows artifact self-test emitted
  its 43-assertion candidate-extractor and 4-assertion materialization
  sub-results but was stopped before a successful process exit; it is not
  acceptance evidence, and no artifact/installer/updater was produced.

The current host inventory still lacks the pinned ASIO SDK archive/extraction
and current-source Show-ASIO bridge artifact required by the physical ASIO
command. It also lacks the licensed artifact decision, named ASIO/WASAPI
operator run, TouchDesigner latency setup, or final package review. The
ASIO rows therefore remain Open with their nonclaims intact. The authoritative
ledger remains `27 Complete / 8 Deferred / 23 Open` (`58` total), and both
ledger validators pass.
