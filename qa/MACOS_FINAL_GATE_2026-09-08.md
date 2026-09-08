# Final macOS DMG gate - 2026-09-08

Base main: `cfac3c324bd866b2947fa15e18042f7a796e9d5b`.
Candidate: `chatgpt/macos-final-gate-20260908`.
Status: implemented; independent review and all required local checks passed;
the candidate-specific macOS CI run and its DMG evidence remain outstanding.
Not promoted to main.

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

The local gate is complete, but no macOS Actions run or real DMG acceptance is
claimed yet. The candidate-specific workflow must still run against the final
candidate HEAD and provide its run/attempt, step outcomes, report, DMG hash and
cleanup evidence.

## Remaining acceptance and handoff

ChatGPT implemented and self-reviewed this candidate without delegating execution.
The older isolated report module had a Luna review, but that is NOT independent
approval of this integrated diff. Review the stable candidate and its tests.
Then rerun the four outstanding checks and dispatch the existing manual
`macos-installer.yml` for this branch through an authenticated GitHub interface.
Do not change the workflow trigger or remove guards to obtain a run.

Record the exact run head SHA, run ID/attempt, validation/acceptance outcomes,
report, DMG SHA-256 and cleanup result. If a Mac-only issue is found, fix its
cause and rerun against the new SHA; do not relax inspection to reuse old success.
The previous successful run 34139676339 predates these changes and cannot prove
this candidate. No new macOS job, Developer ID signing, notarization, Gatekeeper,
M2/macOS 12 execution, hardware, venue or public Release acceptance is claimed.

Application code, Windows ASIO/NDI fail-closed routes, fixed three-screen layout,
product versions and the other dirty worktrees are unchanged. No permanent
background work is added to rendering/audio/DMX. New logic runs only in tooling.
Main promotion remains separate from publishing this candidate source branch.
