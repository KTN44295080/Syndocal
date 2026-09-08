# Selected main integration candidate - 2026-09-08

Status: NOT promoted to main. Final frontend validation and selected-tree native
build/IPC acceptance are outstanding. Main remains `00e8785`.
Candidate branch: `chatgpt/core-integration-candidate-20260908`.

## Selected scope

The user authorized main integration only when the changes appear sound.
Six scoped commits were selected from main: original `d317f13`, `3809edc`,
`269c232`, `ab8e670`, `8a8e530`, and `3627811`. They cover canonical own-property
admission, bounded frontend/native thumbnail work, cooperative render cancellation,
libav seek/catch-up regressions, and an ignored virtual-MIDI test.
The unfinished UI ticket/Channel cancellation (`5fc63f1`) is NOT included.
Neither the unintegrated Mac validator nor standalone report publisher is included.
The original 515 native / 456 frontend commands and 47 canonical operations remain.
Runtime admission policy/count/hash, UI layout, ASIO/NDI and versions are unchanged.
Only stale inventory test totals were reconciled to current exact source sets.

## Results on the selected Rust tree

All Cargo commands used the maintained MSVC 14.44.35207 linker/PATH-first checks,
release mode, locked dependencies and serial tests. No hardware tests were enabled.
- Control-plane: 30 passed / 0 failed / 0 ignored.
- Thumbnail: 17 passed / 0 failed / 0 ignored.
- Video with libav: 193 passed / 0 failed / 8 ignored.
- Ordinary video: 180 passed / 0 failed / 3 ignored.
- I/O: 180 passed / 0 failed / 3 ignored; the new virtual-MIDI case stayed ignored.

The five Rust logs contain zero warning diagnostics. A pre-change native warning
baseline was not rerun; no platform-wide warning delta is claimed. Later changes
were confined to frontend retry/checker code and do not alter this tested Rust tree.
Before the final review fixes, TypeScript, check:release, the 456-command frontend
inventory, and the wrapper (243 assertions / 27 hostile fixtures) passed.

## Review findings and unverified final fixes

One read-only Luna/Codex review received the selected inline source diff; it did
not run tests or modify files. Its concerns included delimiter parsing in the
new offline inventory checker and permanent missing thumbnails after transient
native busy/renderer-wait failures. Missing test files were a review-input omission,
not missing source: the compiled native tests above ran from the selected tree.
The unchanged lack of GUI cancellation remains outside this subset's claims.

The candidate now parses only standalone handler-closing lines, rejecting comments
with embedded delimiters. It passed the exact 515-route hash and 18 negative fixtures.
A focused retry module performs at most one serial retry after 100ms, only for two
exact native transient errors and only while its batch remains current. An explicit
Load Thumbnails action also retries missing entries while retaining valid cache hits.
Existing controller regressions passed after this implementation was added.
Additional bounded-retry, retirement, manual-recovery and cache-reuse cases were
then added to the checker, but the final check:media-thumbnails / tsc / check:release
execution request was service-denied before execution. Those new cases are NOT
recorded as passing. The amended code has not received a second independent review.
Therefore the candidate is saved without promotion, not declared ready for main.

## Baseline debt and native boundaries

The untouched main worktree independently reproduced the existing failures:
`check:frontend-command-routing` expects 450 frontend commands but main has 456;
`check-vj-media-import-access` cannot find its old clip-grid branch marker.
A count-only repair exposed further old facade assumptions, so that incomplete
checker patch was not included. Both baseline failures remain recorded as failures.
A broader source-inspection operation was also service-denied; it is not evidence.

An earlier probe in `target/qa/integration-native-20260908-01/` passed real-WebView
negative paths for the EXCLUDED ticket candidate EXE, hash
`4694427CB3E163000BC6E093384521C77E81076340E76E5700A0941F220675F9`.
Both lanes delivered one ticket; cancel/replay returned false after missing-ID
rejection, and missing-Channel calls were rejected. Its own application and debug
listener exited. This is not running-decoder-stop or selected-tree acceptance.
The separate `integration-native-20260908-final` probe is prepared but NOT run;
its hash placeholder intentionally prevents accidental use before a fresh build.

## Resume boundary

First run the final retry/controller, TypeScript, release and inventory checks;
review any failures without weakening assertions. Re-review the amended diff.
Then build the exact selected tree with the maintained Windows no-bundle wrapper,
record its new executable hash, and run the selected-tree native IPC probe.
Only after those gates and the pending-diff review should main be fast-forwarded.
Evidence is retained under `target/qa/main-integration-20260908/` (Rust summaries,
frontend/baseline logs and independent review). Seven old Mac files retain their
starting hashes and remain outside the commit. No installer, release, device or
venue acceptance, final native build, or main update is claimed by this checkpoint.
