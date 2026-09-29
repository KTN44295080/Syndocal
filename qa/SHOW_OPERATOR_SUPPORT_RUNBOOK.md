# Show operation and support runbook

Status: source-grounded draft, 2026-09-30. Use with the actual venue patch,
approved device list, show file, and named operator. These procedures have not
passed a deployed venue drill. Do not infer physical output from a UI status.

## Before doors open

The operator records the application version, project identity and saved state,
machine role, output ownership, active Blackout state, device names, expected
display/DMX routes, and a contact for the other machine if Standby is used.
Open the intended project and resolve any recovery prompt before arming output.
Verify every show media asset and fixture profile on this machine; a matching
filename alone is insufficient when the stored content hash differs. Inspect
current input, audio, video, DMX, recording and ShowClock status. A configured
route is not proof that a worker is running or that the device receives data.
Keep output disarmed and Blackout engaged until the device and ownership checks
have been signed off by the operator. Save an independent project copy before
the show; retain the original and its recovery checkpoints.

## Output Arm, Standby and Blackout

On the intended Active machine, confirm the selected machine output role and
current ownership before using the local Arm control. On Standby, load/follow
the latest agreed show checkpoint, but leave physical output fenced. Do not
assume a replicated project transfers output ownership. For takeover, first
establish that the former Active output is stopped or safely isolated, stop
synchronization, confirm the replacement project and machine role, then use
the replacement machine's explicit local Arm path. Observe the actual fixtures
and displays before clearing Blackout. If that observation fails, re-engage
Blackout and leave the route disarmed while investigating. The Control > Both
desk has separate `DMX BO`, `Video BO` and `All BO` actions; the active state
is shown by the corresponding `Clear ... BO` label. Clearing Blackout can
expose output and is a separate operator decision from Arm.

## Device loss during a show

For audio, video or DMX loss, note the visible error and time, engage the
appropriate Blackout or safe stop for the affected domain, and check whether
the worker reports held, degraded, failed or cleanup pending. Do not repeatedly
Arm or switch a device while a previous stop/replacement is unresolved. Confirm
the exact replacement device, route and ownership before one explicit retry;
verify physical result separately. If a DMX endpoint or display is missing,
keep it isolated rather than redirecting to an unverified fallback. For a
recording sink fault, preserve the partial file and session status before
starting another recording or choosing a new destination.

## Project corruption and recovery

Stop output safely and preserve the failing `.sdc` file, its backups and
recovery checkpoint before any repair attempt. Record the precise error and
project path privately; avoid placing paths in a shared diagnostic bundle.
Use the app's Recovery/backup inspection to identify the last valid state and
restore into a new file. Confirm the project identity and media/profile
availability after loading, then save and reopen that new file before treating
it as the active show source. Do not discard a recovery checkpoint merely
because a save or load request timed out: reconcile its terminal state first.
The authoritative project publication/restart behavior is documented in
[Project publication recovery](PROJECT_PUBLICATION_RESTART_RECOVERY_2026-09-05.md).

## Missing media or fixture profile

Leave affected layers/output stopped. Inspect the Media Library availability
and the expected content identity, then relink deliberately to the matching
file and re-verify every referencing layer. Rebuild optional thumbnails or
proxies only after the source is available; these caches are not the media.
For a missing profile, use the project's embedded profile or an explicitly
verified replacement; do not silently substitute a same-named fixture.
Save/reopen a new project copy and inspect affected cues before resuming.

## Recording recovery

Read the recording status and retain the target and any partial output. If
finalization is pending or its outcome is unknown, do not submit a new start
or overwrite request to guess the outcome. Stop/finalize only the current
session through its supported local action, inspect the file and status, and
record the disposition before opening a new sink. Keep any ambiguous partial
file for support. The current support claim does not include a venue-tested
recording recovery drill.

## Diagnostic export and support handoff

The operator opens Project > `Export Diagnostics`, reads the package preview,
and chooses whether to continue. The preview lists the four supported JSON
entries and states that project/media files, paths, credentials, labels and
raw crash logs are excluded. Select a destination in the save dialog only
after reviewing it. If export reports an error, preserve the error text and
inspect the destination before trying a new filename; never treat a timeout
as proof that no file exists. The ZIP's SHA-256 establishes integrity, not
authenticity. Send the package only through the show's approved support
channel; note version, time, observed symptom and safe output state separately.
The local backend also has a digest-bound, single-use new-file export route;
it is not an external MCP tool or a substitute for the operator's UI preview.
See [diagnostic package evidence](OBSERVABILITY_DIAGNOSTIC_PACKAGE_CHECKPOINT_2026-09-13.md).

## Update, rollback and shutdown

Do not install an update mid-show. Before a planned update, stop output and
recording, reconcile pending project writes, save and reopen the project, and
preserve the current installer and release identity. Follow the exact signed
artifact, channel, signature and rollback procedure in the
[update runbook](UPDATE_RELEASE_RUNBOOK.md). An offline or unavailable update
endpoint must not change the running show; do not switch channels or downgrade
as an improvised repair. If an update fails, retain its evidence, restore the
approved previous version and reopen the preserved project copy before
re-arming anything.

For emergency shutdown, engage safety Blackout, stop/disable domain outputs,
let recording finalization complete when safe, save or preserve the recovery
state, then close the application and power down devices according to the
venue's electrical procedure. Preserve logs, the diagnostic ZIP, project
copy, partial recordings, device topology, timestamps and operator actions
for post-incident review. Do not delete caches, backups or partial files
while the incident is unresolved.

## Support triage and escalation

| Observation | Immediate support action | Escalate when |
| --- | --- | --- |
| Status is stale, unavailable or reports cleanup pending | Keep the affected output fenced; capture status and diagnostic ZIP. | It cannot reach a known stopped state. |
| Project save/load result is unknown | Preserve original, backup and recovery state; reconcile terminal publication before another mutation. | No valid authoritative copy can be identified. |
| Media/profile identity mismatch | Hold affected layer/cue; require explicit exact-content relink and re-verification. | Matching source/profile is unavailable. |
| Device fault or ownership mismatch | Engage Blackout or safe stop; verify exact device and owner before re-Arm. | Physical result differs from backend status. |
| Recording finalization unknown | Preserve partial target and session status; avoid overwrite/restart. | Target integrity or terminal state cannot be established. |
| Update/signature/channel failure | Leave the show on its last approved build; retain manifest/artifact/error. | Approved rollback cannot be verified. |

Escalation means hand off to the designated show lead and engineering/support
owner with the evidence above. No step here authorizes an untested physical
route, installation, publication, or release acceptance.
