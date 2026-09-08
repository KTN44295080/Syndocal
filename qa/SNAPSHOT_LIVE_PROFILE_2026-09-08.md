# Snapshot live-projection cost — 2026-09-08

Base main: `233b82af2edce870dfa2afae71ce49becb9b5cd1`.
Branch: `chatgpt/snapshot-profile-20260908`; separate worktree from Luna's native
missing-file/retry acceptance. This checkpoint contains tooling and evidence only.
No application/runtime code, IPC, output policy or other worktree was changed.

## Reproduced duplicate work

`App.tsx`'s `applyEngineSnapshotSyncResponse` updates latestEngineSnapshot,
live DMX **display** data and live fixture display values. When a response is
full or syncUiState is true, it subsequently calls `applyAcceptedEngineSnapshot`,
whose first three statements perform those same live updates again.
This does not mean two physical DMX packets are sent: these are frontend setters.

The new offline profiler extracts the actual sync arrow function and the exact
three-statement live-publication prefix from App's TypeScript AST. It executes
that source slice with the actual `snapshotLiveFixtures` implementation and
explicit harness ports for merge/admission/editor effects. It is neither a
native test nor a mounted full-App test. The non-live editor tail is not executed.
Source structure changes fail explicitly rather than silently matching a comment.

| Path | Live fixture projections | Fixture/DMX display publications | Editor applications |
| --- | ---: | ---: | ---: |
| Full response | 2 | 2 / 2 | 1 |
| Delta with full UI application | 2 | 2 / 2 | 1 |
| Live-only delta | 1 | 1 / 1 | 0 |
| Harness rejects ingress | 0 | 0 / 0 | 0 |

## Timing evidence (synthetic frontend workloads only)

Windows Node 22.22.1; 8 active cues and 8 controls per fixture. Thirty warmup
projections precede 200 timed samples per workload. One/two-call measurement
order alternates. Values below are milliseconds for the real pure projection
function; they exclude setter effects, native capture, IPC, locks, DOM and GPU.

| Fixtures | One call p50 | Two calls p50 | One call p95 | Two calls p95 |
| ---: | ---: | ---: | ---: | ---: |
| 64 | 0.0300 | 0.0598 | 0.0509 | 0.1001 |
| 512 | 0.2515 | 0.5051 | 0.5233 | 0.9449 |
| 2048 | 1.0343 | 2.1250 | 1.6179 | 2.9435 |

The checked-in JSON has all samples' summary statistics and raw-worktree UTF-8
source hashes (line-ending-sensitive). Timing is descriptive, not a pass threshold.
Do not turn the two-call/one-call comparison into a claimed product speedup:
no optimization was applied. Inputs are generated in memory, not a representative
operator show. GC, JIT, CPU scheduling and other machine load affect these values.
Inactive cues return the original fixture array; active-cue work is the measured case.

## Executed checks

`node --check app/scripts/profile-live-snapshot.mjs`: exit 0.
`node app/scripts/profile-live-snapshot.mjs`: exit 0; duplicate call counts reproduced.
`node app/scripts/profile-live-snapshot.mjs --output qa/SNAPSHOT_LIVE_PROFILE_2026-09-08.json`:
exit 0; recorded second run, with identical call counts and the timings above.
The tool asserts non-mutated inputs, correct projected values and a single editor
application on the full/UI paths. The injected rejection case checks the harness's
control flow, not real native authority validation. No real IPC policy is replaced.

Run from the repository root; an optional output file must not already exist:

```sh
node app/scripts/profile-live-snapshot.mjs --output NEW_REPORT.json
```

No network, engine worker, application window, native reader, physical device or
output is opened by the profiler. It is not imported by the product or release
startup path and introduces no production polling or per-frame work. No new
package dependency or lockfile update. No compiler ran in this checkpoint, so
there is no new Rust-warning measurement. Review is author self-review only.

## Next bounded repair (not performed here)

Remove the duplicate live publication while preserving the existing sequence of
project/timeline admission, accepted snapshot identity, revision publication,
editor synchronization, draft preservation and final selected/focused state.
Do not replace this with an unbounded cache or assume reference equality permits
skipping all future projections. Regression must cover full, UI delta, live-only
delta, rejected ingress and successive different projects. Then use the Windows
native gate and independent review before promoting a runtime change.

The wider saved-show capture/clone, native read/write-lock wait, delta generation,
serialization/payload and actual-UI workload measurements remain uncompleted.
The native profiler directory-creation request and a saved-snapshot analysis
request were blocked before execution; no native benchmark is claimed. This
frontend-only diagnostic does not close a whole completion-ledger row.
Luna's real missing-file/recovery test files and application session are unmodified.

Final tooling checks: exclusive output collision returned nonzero/EEXIST while
the previously recorded JSON remained byte-identical; an unknown CLI argument
was rejected. Both checks passed. Completion-ledger and Q1-Q4 validators passed
without altering any acceptance row. `git diff --cached --check` passed before
commit. This evidence branch is separate from main while Luna owns its current
integration/acceptance work; no concurrent main update is attempted here.
