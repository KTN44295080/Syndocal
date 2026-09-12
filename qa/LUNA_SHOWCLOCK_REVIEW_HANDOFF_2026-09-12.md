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
