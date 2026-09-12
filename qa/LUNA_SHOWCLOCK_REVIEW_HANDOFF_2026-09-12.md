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
5. Then continue ShowClock per master G9: estimator/bounded slew/Hold/STALE
   deterministic simulator before LAN adapter, followed by action scheduling,
   generation coupling, output ownership and IPC/UI. Freeze concrete thresholds,
   pairing/key/restart policy, ports, action horizon/capacity/late behavior.
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
