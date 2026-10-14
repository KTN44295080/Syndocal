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
3. **Two-PC lease, supersede and takeover take** (implemented 2026-10-14:
   `recording_lease.rs`, `takeover_recording.rs`, `configure_takeover_recording_v1`)
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
   takeover-recording folder in the Standby sync panel. Implemented:
   - the Record button's reason and end detail in the recording bar;
   - Pending or Blocked takeover state in the recording bar;
   - a "Takeover recording folder" row (Browse, Clear, Cancel) in
     Active / Standby.
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

## Phase 3 design notes (2026-10-14, as built)

- **Lease file.** A take that observes Standby sync running as Primary binds
  that sync folder and session. It rewrites
  `syndocal-recording-lease-<session>.json` every second with a sequence
  number, take id, output, frame rate and audio flag.
  - The lease TTL is 3 s, shorter than the 5 s standby stale threshold.
  - Binding counts as a renewal.
  - A restarted sync session re-binds the take. The old session's lease
    is left to go quiet, never marked ended while the take still runs.
    Receipts are checked in every folder the take was bound to.
  - When the take really ends, every lease it renewed says `ended`.
  - The binding survives stopping sync, so a Standby never sees a silent
    but live recorder.
- **Self-fence.** If the last successful renewal is older than the TTL on the
  recorder's own clock, the frame loop ends the take as `ownership_revoked`
  with the reason in `end_detail`. It also ends the take on:
  - a `syndocal-recording-takeover-*.json` receipt naming any session the
    take was bound to;
  - a split-brain that persists for 10 s. Both Primaries fence. A starting
    Primary briefly reports old sessions of a reused folder as active, and
    that must not end a take.

  A take that is already superseded, or whose first lease cannot be
  written, never starts.
- **Predecessor state.** There is no cross-PC clock comparison. The new
  owner sees each predecessor as one of:
  - `Ended`: its lease says so;
  - `Quiet`: its lease has stayed unchanged, or absent, for 5 s on the new
    owner's own clock;
  - `Recording`;
  - `Unknown`: the lease cannot be read.

  `Quiet` counts as ended only once the takeover receipt telling the old
  Primary to stop has been written; a paused PC must not be mistaken for a
  stopped one. This assumes the sync folder makes writes visible within
  that margin, as the existing 5 s heartbeat already does. A cloud-synced
  folder with long latency is not a valid sync folder.
- **Takeover take.**
  - Take Over writes an immutable `syndocal-recording-takeover-<id>.json`.
    It names every superseded session and carries the predecessor leases
    still recording. Unreadable predecessor leases are waited for too.
  - A receipt counts as written only when it reads back intact.
  - A failed write is retried. Receipts older than 7 days are pruned.
  - After Arm grants video, a takeover recorder waits until every
    predecessor has ended. It then starts the first predecessor's output,
    frame rate and audio choice, with no dialog, into the configured
    takeover folder.
  - Until then the status is `Pending` with the reason. It is `Blocked`
    when there is no folder, the folder is missing, or the receipt has not
    been written while a lease is only quiet. Unreadable leases are
    reported; nothing is silent.
  - Opening another project, returning to Standby or the operator's Cancel
    ends the plan.

## Phase 5 runbook: physical two-PC acceptance (owner)

Setup:
- PC1 and PC2 run this build.
- Both share one LAN sync folder (SMB, not a cloud-synced folder).
- Video output 1 exists in the project.
- On PC2: Active / Standby › Takeover recording folder › Browse picks a
  local folder.

Each step lists what to do, then the expected result.

| # | Do | Expect |
|---|----|--------|
| A | PC1: Start Sync as Primary, Enable Output, Record Output. PC2: Start Sync as Standby. | PC1 records. PC2's Record button is disabled with "This PC runs as Standby; only the Primary records." The sync folder has `syndocal-recording-lease-<PC1 session>.json`, rewritten every second. |
| B | Kill PC1 (Task Manager, or pull power). On PC2: Take Over, then Enable Output (Arm). | PC2's takeover state goes "Waiting for the previous Primary's take to end", then starts about 5 s after PC1's lease stopped changing. A `…-takeover-<ms>.mp4` file appears in PC2's folder. PC1's file is not published by us (its partial stays). On the next PC1 start, its receipts show the take as `interrupted_unknown`. |
| C | Repeat A. With PC1 alive, PC2: Take Over with the "I have fenced…" confirmation, then Arm. | Within about 1 s PC1's take ends: "Ended: Another PC took over output (…)". Its file is published. PC2's takeover take starts right after PC1's lease says `ended`. No two takes overlap by more than about 1 s. |
| D | Repeat A. Disconnect PC1 from the sync folder (unplug its network) for more than 3 s. | PC1's take ends with "The recording lease in the sync folder could not be renewed: …". Its frames are published locally. |
| E | Repeat A. Cut PC1's power mid-take, then restart PC1. | The receipts in `%LOCALAPPDATA%/…/recording-receipts/` show the take as `interrupted_unknown`. Recording works again. |
| F | Start Sync as Primary on both PCs on the same folder, both recording. | Both takes end about 10 s after both sessions are seen: "Two Primary sessions stayed active…". |
| G | On PC2 after B: Open a different project while the takeover is Pending (re-run B with Arm withheld). | Takeover state "Cancelled · Another project was opened after Take Over…". |

Record the results under COV-RECORDING-001 and DEC-RECORD-OWN-001 in the
completion ledger.
