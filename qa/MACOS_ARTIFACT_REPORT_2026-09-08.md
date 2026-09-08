# macOS artifact report publication — 2026-09-08

Base: `8a8e530c6801a5399f0d71c93dcead343a35ad0a`.
Branch/worktree: `chatgpt/macos-artifact-gate`, separate from the existing
`chatgpt/macos-artifact-validation` worktree. The original worktree had concurrent
uncommitted thumbnail-cancellation changes and an active Cargo test; none was
edited, staged, committed or terminated by this lane.

Scope: one standalone report-publication module and its tests. This is NOT a
completed macOS validator or an integrated workflow change.

## Implemented boundary

`app/scripts/macos-artifact-report.mjs` prepares evidence in a uniquely named,
exclusively created sibling file. Its asynchronous write/sync/close phase records
`incomplete`, not an accepted pass. After yielding for pending callbacks, the
module rechecks interruption, synchronously writes and syncs the bounded final
record, then publishes via an exclusive hard link. Existing final evidence is
never replaced; failures before the link leave no partial final record.
An interruption observed before commit changes the record to failure.

The final link is a point-in-time checks record, not proof of a successful CI job.
The record requires the matching workflow's success as a separate condition.
An OS kill, signal or runner cancellation after commit cannot retroactively change
that record. Consumers must require the exact final path AND successful/non-cancelled
validation/job outcomes; a pending file or a standalone `status: pass` is insufficient.

## Responsibility and limitations

The module owns publication only; report/check policy, subprocess lifecycle,
Mach-O inspection and DMG mounting remain outside it. The caller supplies the
already-finalized check result. The 64 MiB report limit bounds offline CLI I/O;
no application startup, rendering, audio or DMX-loop work is added.
Successful publication removes the staging alias before returning. If removal
fails, publication throws: a linked checks record can remain, but it cannot be
accepted without successful publication/CI outcomes. Failure evidence is retained.
Unused reservation objects do not open files or hold handles.
This assumes a trusted owned evidence directory; it does not claim hostile
filesystem containment, whole-machine crash durability or parent-directory fsync.

## Executed tests

`node --check app/scripts/macos-artifact-report.mjs`: exit 0.
`node --test app/scripts/test-macos-artifact-report.mjs`: initially 16, finally 19 passed, 0 failed,
0 skipped on Windows / Node 22.22.1. Tests cover atomic final visibility,
interruption before/during asynchronous preparation and immediately before commit,
earlier check failure, write/sync/close/link faults, exclusive publication collisions,
one-use reservation and unserializable records. No product application was launched.

Logs: `target/qa/macos-gate-20260908/report-tests-01.log` and `report-tests-02.log`.
This does not establish macOS execution or DMG acceptance. No Rust compiler,
Windows native build, hardware output, Apple signature or notarization was run;
no compiler-warning baseline/delta or successful Mac CI run is claimed.

## Not integrated; other validator blockers remain

The original validator's survival path was reproduced with three additional
regressions: non-zero termination, SIGSEGV and SIGKILL after the observation
interval were incorrectly accepted. The unmodified process code produced
48 passed / 3 failed in the expanded 51-test suite. Its log is retained at
`target/qa/macos-gate-20260908/tests-before.log`; the new regression source stays
with the separate uncommitted validator candidate.

Two requested tool operations were rejected by the service: the process-exit
classification patch, and the caller/workflow integration patch. Neither was
reissued or delegated for execution. Hash comparison confirms the process file,
caller and workflow still equal the copied starting candidate. Therefore this
standalone module has no effect on the current installer workflow yet.

The original seven macOS candidate files were copied without modifying their
source worktree; `source-hashes.json` records their original SHA-256 values.
This checkpoint does not stage those files or any thumbnail changes. Do not call
it a completed final-DMG gate. The remaining acceptance sequence is explicit:
fix the three abnormal-exit regressions, integrate final-report publication with
workflow success/cancellation checks, rerun the complete validator tests, then
run the exact candidate on a macOS runner and retain DMG/hash/run evidence.
No new macOS workflow was dispatched and no public release was created.

## Independent review and disposition

Implementation and test execution were performed directly by ChatGPT. Under the
user's conditional authorization, one read-only Codex CLI / `gpt-5.6-luna` review
received only the two new source/test files inline. It was instructed not to use
tools or execute edits. Exit 0 and its final review are retained in
`report-review.log`, `report-review.txt` and `report-review-exit.json` in the evidence
area. No rejected process or workflow operation was delegated for execution.

The reviewer flagged the retained writable staging alias and abandoned open
reservations. The final implementation removes the alias before successful return
and opens the staging file only inside publish. Three new regressions verify
unused reservations, independent later staging-path writes and fatal alias-removal
failure. These address the two actionable lifetime findings.

Its proposed pass during the second interruption rewrite was not accepted as
stated: that branch first sets `status = fail`, then rewrites those failure bytes.
The pre-link interruption regression exercises that path and confirms failure.
This is not a claim that uncatchable termination or interruption after commit
can be recorded retroactively; the separate workflow-success requirement remains.

The amended diff was self-reviewed and passed 19 tests. A second independent
review of the amended diff was not run; do not describe it as independently
approved or release-ready. This standalone subset is saved separately from the
unintegrated validator and its three known failing process-exit regressions.
