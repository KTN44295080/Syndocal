# Final macOS DMG gate - 2026-09-08

Base main: `cfac3c324bd866b2947fa15e18042f7a796e9d5b`.
Candidate: `chatgpt/macos-final-gate-20260908`.
Status: implemented; independent review, all required local checks, and the
candidate-specific macOS CI/DMG gate passed. Not promoted to main.

## Bounded change

The manual single-job workflow now validates the app extracted from the final
DMG, rather than the app used to build the image. It checks source/run provenance,
image integrity, bundle identity/minimum OS, arm64 Mach-O dependencies, signature
integrity, eight-second process survival, cleanup and checksum publication.
The final report is exclusively published through the prior report-publication
module; success requires removal of its writable staging alias.

A separate acceptance step checks the exact commit/run/attempt, every required
check, normal observed process termination, removed staging path, one DMG/sidecar
pair and the current file hash. Upload requires successful validation/acceptance
steps and a non-cancelled workflow state. Failure logs are retained separately.
A JSON pass or uploaded artifact alone is NOT evidence of whole-job success.

Process I/O, inspection, policy, publication, upload acceptance and CLI wiring
remain separate modules. Forced termination, crash signals, nonzero exits,
timeouts and unreaped children fail validation. The process runner owns only
its direct child; no process-tree or physical-device acceptance is claimed.

## Executed evidence

Windows / Node 22.22.1. Evidence directory:
`target/qa/macos-final-gate-20260908/resume-01/`.

- Previously recorded baseline: 67 passed / 3 failed among 70 tests. The three
  failures accepted code 7, SIGSEGV and SIGKILL after the observation interval.
- `tests-existing.log`: 70 passed / 0 failed / 0 skipped after the process fix
  and report integration; the three abnormal-exit regressions now reject.
- `tests-integrated.log`: 147 passed / 0 failed / 0 skipped across the validator,
  report-publication, acceptance and workflow-contract files. Includes real Node
  children and temporary-file hashing plus simulated Mach-O/CI report metadata.
  Temporary fixture bytes are not real DMG acceptance.
- `syntax-results.json`: all ten JavaScript files passed `node --check`.
- The workflow contract checks manual trigger, one job, ordered exact report
  paths, preserved test failures and conditional upload. This is a focused
  structural checker, not GitHub's workflow parser. PyYAML was unavailable;
  general YAML parser validation was not performed.
- Frontend dependencies were installed from the existing offline pnpm store
  with the frozen lockfile; no dependencies or lockfile content were changed.
  pnpm reported its existing ignored-esbuild-build-script policy; it was not
  changed and is not a compiler-warning or runtime acceptance measurement.

## Independent review and residual local checks

The stable 13-file diff was independently reviewed against base main. The review
confirmed that abnormal exit, timeout, interruption and unreaped-child states
remain failures; direct-child ownership is retained until `close`; unknown mount
metadata, failed detach and still-mounted volumes preserve the temporary
directory; report publication keeps a pending/final boundary; final report
acceptance requires the exact commit/run/attempt, complete check inventory,
non-cancelled validation outcome, one DMG/checksum pair and matching SHA-256;
and the workflow keeps a manual single job with validation/acceptance-gated DMG
upload plus always-retained evidence. No assertion or environment guard was
removed.

Fresh evidence is under `target/qa/macos-final-gate-20260908/resume-02/`:

- The integrated artifact/policy/report/acceptance/workflow suite passed
  `147/147`, with zero failures, skips or cancellations.
- `check:tauri-build-wrapper` passed 243 assertions and 27 hostile fixtures.
- `check:completion-ledger`, `check:q1-q4-ledger`, and `git diff --check` passed.
- The first `check:release` attempt correctly stopped because this worktree had
  no staged Windows runtime DLLs. `prepare:runtime-libs` also correctly rejected
  the available external SDK because its DLL bytes did not match the pinned
  inventory. For the final rerun, the exact seven pinned DLLs were copied from
  the already verified base-main `target/release` artifact after SHA-256/size
  verification; they are ignored build artifacts, not source changes. The final
  `check:release` then passed all subchecks, including the exact 515-command
  inventory and 18 negative fixtures. The initial environment failure is retained
  in `check-release.log` and is not counted as a candidate code failure.

The local gate and candidate-specific workflow gate are complete; the detailed
Actions and DMG evidence follows.

## Candidate macOS Actions evidence

The implementation candidate `c4144d0dd092a8d9f2ad8f79d6b6816be3d050fc` was
run by the existing manual workflow without changing its trigger or guards.
Run `34188473755`, attempt `1`, job `101941484394`, and the workflow checkout
all report that exact head SHA. The run and job concluded `success` and were not
cancelled. Every required and post-job step completed successfully, including
the validator test, application build, final-DMG build, extracted-app
validation, acceptance, validation-evidence upload, and DMG upload.

The downloaded evidence is retained under
`target/qa/macos-final-gate-20260908/resume-02/actions/`:

- Final report: `status: pass`, `interrupted: false`; all ten checks are `pass`.
- Report/run identity: commit `c4144d0...`, run `34188473755`, attempt `1`.
- DMG: `Syndocal_1.2.0-alpha.69_arm64.dmg`, 31,505,009 bytes,
  SHA-256 `a795234d4d3db6347dded02394bd836b9f9f84a95ab14960576eb8c16315d277`.
- Process survival: 8,010 ms observed, reaped, requested SIGTERM, no abnormal
  exit or signal.
- Cleanup: exact temporary mount detached and temporary directory removed;
  final report staging alias was absent before acceptance.
- Acceptance re-read the DMG and sidecar from the upload directory and matched
  the same hash. Evidence artifact ID `10041501120` and DMG artifact ID
  `10041502006` were both uploaded successfully.

This is final-DMG structural/signature/process evidence on a GitHub-hosted
macOS 15 arm64 runner. It does not claim M2 hardware, macOS 12 execution,
native UI responsiveness, device I/O, Developer ID trust, notarization,
Gatekeeper, or venue acceptance.

## Remaining acceptance and handoff

ChatGPT implemented and self-reviewed this candidate without delegating execution.
The older isolated report module had a Luna review, but that is NOT independent
approval of this integrated diff. Review the stable candidate and its tests.
The stable candidate has now received the independent review, all four local
checks, and the authenticated manual `macos-installer.yml` run recorded above.
The workflow trigger and guards were not changed to obtain the run.

The exact run head SHA, run ID/attempt, validation/acceptance outcomes, report,
DMG SHA-256 and cleanup result are recorded above. If a future Mac-only issue
is found, fix its cause and rerun against the new SHA; do not relax inspection
to reuse old success.
The previous successful run 34139676339 predates these changes and cannot prove
this candidate. No new macOS job, Developer ID signing, notarization, Gatekeeper,
M2/macOS 12 execution, hardware, venue or public Release acceptance is claimed.

Application code, Windows ASIO/NDI fail-closed routes, fixed three-screen layout,
product versions and the other dirty worktrees are unchanged. No permanent
background work is added to rendering/audio/DMX. New logic runs only in tooling.
Main promotion remains separate from publishing this candidate source branch.
