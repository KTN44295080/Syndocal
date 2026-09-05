# Project publication restart recovery

Base: `d009d5b80902637faf295dc5a1ef6ff2f5d410e5`, branch
`codex/syndocal-v1.2`. Unity live-output acceptance is explicitly deferred by
the user. This tranche repairs the separately observed autosave owner error.

## Cause and change

- An old renderer's durable autosave request survived restart. Missing native
  receipt was incorrectly treated as permission to resend that request under
  its retired owner. The native owner guard rejected it. Failed backups did not
  advance scheduling, so each10-second timer could repeat the error and replace
  unrelated operation feedback.
- The new `resolve_missing_project_publication_v1` command verifies the invoking
  owner/window, the old owner's retirement and exact journal state. It returns
  an existing terminal, exact latest-ACK evidence, or durably records an Abandoned
  terminal only for a proven never-reserved next request. It never rewrites an
  owner, skips a sequence number, or writes a project/backup/target/staging file.
- Unknown old requests, conflicting shapes, sequence gaps, live old owners,
  pending requests, capacity limits and persistence errors remain visible errors
  and preserve the local intent. Existing journal schema is sufficient; the new
  response has its own strict version1 tagged contract.
- Frontend consumes exact evidence through the existing terminal/ACK queue.
  Already-acknowledged old images are never applied to the current project.
  After proven abandonment/ACK, a requested new save captures current authority
  and allocates the next identity. No additional confirmation is needed to settle
  an operation proven never dispatched. Existing pending-owner adoption remains.
- Old-owner autosave Success is also ACKed before one fresh current-owner backup:
  its old image cannot mark the present autosave signature as saved. This applies
  to both the usual receipt query and a terminal found by missing resolution.
  Manual-save terminal behavior is unchanged. Two additional E4 cases prove
  old ACK precedes the single fresh capture and current-owner result.
- Autosave captures are single-flight. Desktop backup attempts, including failed
  ones, are separated by60 seconds. Repeated identical errors are not repeatedly
  posted; actual checkpoint/backup success resets that suppression. Detached panes
  retain browser checkpoint behavior but do not run desktop backups or shared
  publication startup recovery.
- Fixed the existing raw invoke escape at the project-transaction controller
  boundary with a typed finite-command adapter, allowing the exact invocation
  inventory to verify the new command without widening that controller's port.
- Initial native startup exposed an integration omission: adding the command
  changed the frozen Tauri count/hash, invalidating the whole admission table and
  blocking even owner registration. Updated the exact510-command fingerprint,
  classified resolution as RecoveryMaintenance and synchronized affected registry
  counts. Admission tests explicitly check both resolution and owner registration.
  The failed startup is not accepted as native verification.

## Evidence

- `node app/scripts/check-project-publication-e4.mjs`: PASS. Actual controller
  covers unknown retired request retention, resolved abandonment and next-owner
  save, ACK-only settlement without old-image application, malformed/future
  replies, and lost ACK preservation, in addition to existing publication cases.
- `node app/scripts/check-project-autosave-coordinator.mjs`: PASS. Single-flight,
  failure cooldown, exact-error suppression/reset and main/pane wiring.
- `node app/scripts/check-frontend-tauri-invokes.mjs`: PASS, exact450 commands.
- `node app/scripts/check-backend-operator-contract.mjs`: PASS, exact510-command
  fingerprint. `node app/scripts/check-frontend-command-routing.mjs`: PASS,
 132renderer/31server-authoritative classifications. Its stale131 expectation
  already disagreed with HEAD's132-entry classification and was synchronized;
  exact frontend/backend set equality is still checked.
- `pnpm --dir app exec tsc --noEmit`: PASS, warnings0.
- `node target/qa/recording-atomic-20260905/run-native.mjs cargo test -p syndocal
  --locked missing_publication_ -- --nocapture --test-threads=1`:3 PASS,
  0failed,1700filtered, warnings0. Exact MSVC pin and where-first verified.
  Evidence `target/qa/snapshot-cleanup-20260905/missing-publication-native-tests.log`.
  Tests cover durable sequence/ACK/lost replies, unknown or conflicting requests,
  live/mismatched owners, owner lock held through persistence, failed-write
  nonmutation, and actual journal reload. No physical device is opened.
- Native/frontend implementation and autosave coordination independently reviewed:
  ACCEPT. First-party warning baseline0/current0/delta0 in configurations above.
- Read-only live pre-fix evidence: old autosave intent request192/next193 persisted
  with no queued ACK (`publication-before-fix.json`). The same read confirmed the
 4K project had loaded successfully; current live output config was SpoutSender
  foreground3840×2160. Unity receipt remains unverified and deferred.

## Native integration acceptance

- Stable-source `node target/qa/recording-atomic-20260905/run-native.mjs cargo test
  -p syndocal --locked control_plane::tests -- --nocapture --test-threads=1`:
  30 PASS, 0 failed, 1673 filtered, compiler warnings0. Final evidence:
  `target/qa/snapshot-cleanup-20260905/publication-recovery-admission-tests-debug-final.log`.
  Earlier attempts exposed stale aggregate expectations; these were synchronized
  with the additional native/frontend sources and maintenance classification.
- `pnpm --dir app tauri build --no-bundle`: PASS, release compilation2m30s,
  first-party warnings baseline0/current0/delta0. Exact MSVC pin and PATH-first
  verified. Log: `publication-recovery-admission-native-build.log` in that evidence directory.
- Diagnostic native startup PID37988 successfully registered its owner. Read-only
  evidence `publication-after-fix.json` shows the existing request192 settled by
  normal startup recovery: intentnull, ACKnull, next193 unchanged. No local-storage
  edits or mutating diagnostic IPC were used. Output ownership returned Ready,
  errornull, with outputs disarmed; this is not external output acceptance.
- Restarted normally without the diagnostic port: PID27020, one responsive,
  maximized Syndocal main window. Executable SHA256:
  `AB59501F44E0BB294377ACB619C1454BF38EFA21550C785D999A3773A2B383ED`.
  Evidence `publication-recovery-normal-launch.json`. No diagnostic observer remains.
- Next: user-driven opening of the Unity4K test project and Art-Net/Spout reception
  in Unity. External video cadence, crop and physical lighting remain unverified.
- Preserve unrelated `app/scripts/check-viewport-containment.mjs` unchanged; it
  is excluded from this checkpoint. No Unity files or show-output actions changed.
