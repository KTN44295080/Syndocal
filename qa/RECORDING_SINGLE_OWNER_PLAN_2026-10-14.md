# Single-owner recording plan — 2026-10-14

Implements DEC-RECORD-OWN-001 (accepted 2026-10-14, `5f0b382c`): one
recording owner, with takeover. It closes part of COV-RECORDING-001.

## Owner answers (AskUserQuestion, 2026-10-14)

1. **Recording requires video output ownership on every setup**, including
   a single PC without Standby sync. Recording cannot start before video
   output is armed. Opening a project, a role change or entering Standby
   ends the current take; the take is finalized, not abandoned.
2. **The takeover take starts automatically** when the new owner arms video
   output. It records into a pre-configured local folder, with no save
   dialog. It waits only while there is no evidence that the old PC's take
   has ended.

## Current state (survey)

- Recording runs Idle → Preparing → Recording → Finalizing →
  Complete|Fault. It uses one worker per process, staged artifacts and a
  recoverable publication.
- It is deliberately outside the output ownership gate. That is the rule in
  `qa/SYNDOCAL_2PC_TRANCHE1_OUTPUT_OWNERSHIP.md`, and `check-output-ownership`
  asserts it. Nothing stops a take on Standby or on a role change.
- Two-PC sync works through a shared folder: manifests and checkpoints
  every 2 s, stale after 5 s, split-brain detection. Take Over goes through
  the lease-authorized OutputControl path; outputs stay unarmed until Arm.

## Phases

1. **Local ownership gate**
   - Admission requires a Ready gate with video allowed. Refuse start on:
     - StartupDenied, Transitioning, Activating or Failed;
     - an epoch change;
     - Standby sync role;
     - split-brain;
     - a poisoned lock.
   - Check admission before the save dialog and again right after it.
     The first frame's epoch permit covers the short gap before the worker
     starts.
   - The frame loop holds a video permit only around snapshot capture. A
     refused permit ends the take with `OwnershipRevoked`, and the take is
     finalized and published.
   - Status gains owner epoch, end reason and admission (the take id comes
     with the receipts in phase 2).
   - Project open, swap and Undo/Redo retire output, so they end a take too.
     Split-brain and another PC's Take Over end a running take only in
     phase 3.
   - Invert the old exemption in the checker and the 2PC document.
2. **Local receipt journal** (implemented 2026-10-14, `recording_receipts.rs`)
   - Immutable V1 receipts: `TakeStarted` is written before Recording,
     `TakeEnded` at the end.
   - A start whose receipt cannot be written is refused.
   - At startup, every orphaned `TakeStarted` gets one
     `TakeEnded{InterruptedUnknown}`; the partial is never published.
   - As built:
     - Receipts live in app-local `recording-receipts/` and are created
       with `create_new` plus a sync; a failed write removes the file.
     - A worker panic still writes a `fault` end through a drop guard.
       Its `published` and `frames_written` stay null, because the outcome
       is unknown; interrupted ends use null too.
     - Pruning keeps the newest 1,000 completed takes, never a take
       recovered in the same pass, and removes lone end receipts. It is
       best effort.
     - More than 50,000 receipts refuses recording with a restart hint.
3. **Two-PC lease, supersede and takeover take**
   - A 1 Hz recording lease in the shared folder while Primary is
     recording, with TTL < standby stale. The frame loop self-fences on the
     lease deadline.
   - The takeover writes a `TakeoverStarted` receipt with the
     predecessor's state.
   - After Arm, the takeover take auto-starts into the configured folder
     once the predecessor is clear (owner answer 2). Otherwise it is
     Pending, never silent.
   - A forced takeover without evidence needs an explicit operator
     confirmation.
4. **UI:** owner or blocked reason, take id, end reason, Pending, and the
   takeover-recording folder in the Standby sync panel.
5. **Physical two-PC acceptance:**
   - Standby cannot record.
   - Kill the Primary, then Take Over, Arm, and check that the new take
     does not overlap.
   - Force a takeover while the Primary is alive; the Primary ends its take.
   - Cut the Primary's share access; it self-fences.
   - Power-loss receipt.
   - Two Primaries both fence.

Phases 1 and 3 change runtime and safety behaviour, so each needs an
independent review and the Windows native gate.

## Phase 3 design notes (2026-10-14)

- **Lease file.** A take that observes Standby sync running as Primary binds
  that sync folder for the rest of the take. It rewrites
  `recording-lease-<session>.json` every second with a sequence number,
  take id, output, frame rate and audio flag.
  - The lease TTL is 3 s, shorter than the 5 s standby stale threshold.
  - The binding survives stopping sync, so a Standby never sees a silent
    but live recorder.
- **Self-fence.** If the last successful renewal is older than the TTL on the
  recorder's own clock, the frame loop ends the take as
  `ownership_revoked`. It also ends the take on:
  - a `recording-takeover-*.json` receipt that supersedes its session;
  - split-brain (both Primaries fence).
- **Predecessor clear.** No cross-PC clock comparison. The new owner treats
  the predecessor as clear when any of these holds:
  - its lease says `ended`;
  - its lease is absent;
  - its sequence has not changed for TTL + 2 s on the new owner's own clock.

  This assumes the sync folder makes writes visible within that margin, as
  the existing 5 s heartbeat already does. A cloud-synced folder with long
  latency is not a valid sync folder.
- **Takeover take.**
  - Take Over writes `recording-takeover-<new session>.json`. It names the
    superseded sessions and carries the predecessor's lease, if any.
  - After Arm grants video, a takeover recorder waits for the predecessor to
    be clear. It then starts the same output, frame rate and audio choice,
    with no dialog, into the configured takeover folder.
  - Until then the status is `Pending` with the reason. Without a configured
    folder it is `Blocked`, never silent.
