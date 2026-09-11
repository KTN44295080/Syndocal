# AI7 event-gap and snapshot convergence revalidation — 2026-09-11

This is one bounded AI7 proof slice on current `main`. It revalidates the
existing snapshot ingress and publication behavior; it does not add a new
route, retry policy, or product feature.

## Scope

- source HEAD: `f75ad6d40c9f41ed3cb3a53e0dcddbd7a9ec5c54`
- worktree: clean before the run
- owner: current-main QA lane
- evidence directory: `target/qa/ai7-event-gap-current-main-20260911-01/`

## Evidence

| Check | Result |
| --- | --- |
| `node app/scripts/check-snapshot-runtime-watermark.mjs` | PASS — full/delta/poll/canonical monotonic ingress, stale-result rejection, and watermark fencing |
| `node app/scripts/check-snapshot-live-publication.mjs` | PASS — full, UI-delta, live-only, rejected, and successive publication paths |
| `git diff --check` | PASS before this documentation-only checkpoint |

The first checker preserves the existing rule that a stale or out-of-order
response cannot replace newer visible state. The second confirms that accepted
full/UI-delta/live-only paths publish through the existing seam, while rejected
and successive responses do not produce an unintended state transition.

This closes only the software event-gap/snapshot-convergence evidence slice.
It does not close AI7 as a whole: 10,000-call realtime saturation, sidecar
termination during an active show, recording/file/project replacement
reply-loss, release-native external-client acceptance, hardware, and other
adversarial proof remain open or externally bounded. No physical output was
enabled.

## Current-main event-gap and snapshot convergence revalidation — 2026-09-12

The same bounded AI7 proof slice was rerun against current `main` at source
HEAD `7d58509272c25923571fe299d7735fd71bc3e961`; `origin/main` matched before
the run. No snapshot route, retry policy, or publication behavior was changed.

| Check | Result |
| --- | --- |
| `node app/scripts/check-snapshot-runtime-watermark.mjs` | PASS — full/delta/poll/canonical monotonic ingress, stale-result rejection, and watermark fencing |
| `node app/scripts/check-snapshot-live-publication.mjs` | PASS — full, UI-delta, live-only, rejected, and successive publication paths |
| `git diff --check` | PASS |

The current run preserves the rule that stale or out-of-order responses cannot
replace newer visible state, and that rejected/successive responses do not
create unintended publication transitions. This remains one software proof
slice only. AI7 stays Open for realtime saturation, sidecar termination
during an active show, recording/file/project replacement reply-loss,
release-native external-client acceptance, hardware, and other adversarial
matrices.
