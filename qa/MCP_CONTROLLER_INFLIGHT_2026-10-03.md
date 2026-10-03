# Native R4/R5 interruption before broker completion

Branch `codex/showclock-review-20260912`, base `eb8e3387`.
This checkpoint extends the completed-terminal
[restart proof](MCP_CONTROLLER_RESTART_2026-10-03.md) with actual unresolved
external requests at three production broker boundaries. The tested families
are R4 Lighting master and R5 diagnostic ZIP export.

## Ownership and test boundaries

`check-native-lifecycle.mjs --controller-inflight` uses the common helper's private
fixture/project, ephemeral `127.0.0.1` receiver, independently authored full
512-channel oracle, authenticated stdio clients and final project-replacement
disarm. A focused `native-controller-inflight.mjs` owns the interruption matrix;
the runner owns exact-path process termination/relaunch, pairing, credentials,
revocation and cleanup. Ten conflicting opt-in lanes reject before executable
selection or launch.

The test retires the automatic bridge renderer through the existing native
`agent_bridge_register_v1` boundary, as the maintained post-claim revocation
probe already does. It submits a real authenticated MCP request, claims the
immutable queued request through `agent_bridge_claim_v1`, and where selected
executes the existing `agent_bridge_execute_native_v1`. It withholds
`agent_bridge_complete_v1`. This proves the external broker still reports
`pending` even when the native domain effect is visible. It introduces no DOM
action, invoke patch, synthetic engine, product hook, IPC change or policy change.

| Family | Queued before claim | Claimed before execution | Native effect before broker completion |
| --- | --- | --- | --- |
| R4 Lighting master | Full image unchanged | Full image unchanged | Explicit master 500 produces half image |
| R5 diagnostic ZIP | Destination absent | Destination absent | New ZIP's hash, size and header verified |

Each of the six cases proves the pending broker state, kills the exact QA
process, observes actual UDP sender silence and restarts with fresh
process/session authority. Fresh pairing, grants and reused owner text cannot
reclaim the lease or resume output. Previous UUID lookup is `unknown` without a
result. Under the same authenticated incarnation, exact immutable replay also
stays `unknown`; if incarnation differs, its persisted shape conflicts. Changed
arguments under the old UUID always conflict. A fresh UUID carrying the old R4
process fence rejects `stale_fence`. These are deliberate negative replay
injections, never an automatic retry of an uncertain mutation.

Unexecuted R5 requests leave the destination absent across restart. A committed
ZIP remains byte-identical; conflicting replay cannot create a replacement
target. Native execution is single-use before broker completion. Explicit
private project reload and fresh acquisition remain silent, old unclaimed lease
operations reject, and a separate explicit Arm resumes the independent full
image with fresh packets. Final project replacement closes both runtime gates
and stops the owned sender. Private diagnostic files are removed with the
common helper's individually owned directory.

## Request-ID diagnostic

The first probe and its diagnostic repeat passed queued admission, actual
restart, unknown lookup/exact replay and changed-shape conflict, then stopped
on the stale-lease rejection assertion (`invalid_request` versus expected
`forbidden`). Source inspection found two test-side numeric request-ID
allocators. The manual old-fence probe advanced the common native high-water
identity fence; the common helper's later, lower request ID was correctly
rejected before lease authorization. `control_plane_runtime.rs` rejects a new
identity at or below that high-water mark as `InvalidRequest`.

The probe now uses the common helper's one request-ID allocator. The original
`forbidden` acceptance assertion remains intact, as do old-fence, unknown and
shape-conflict assertions. No product check, lease policy, high-water fence or
error mapping was weakened. Raw negative results are retained with their
original harness identities.

## Actual native result

[The final current-source probe](artifacts/native-controller-inflight-2026-10-03.json)
passes 35 actual check groups: four common output/setup/cleanup groups,
four groups for each of six interruption cases, and seven base lifecycle groups.
The receiver validates all 893 native packet headers and compares the complete
512-channel payload for 581 new frames across admission, pending, explicit
re-Arm and initial-Arm stable phases.
No retryable read overload occurs. All six prior-request lookups and exact
same-incarnation replays return `unknown` with no execution/result.

| Case | Native PID terminated | Fresh native PID | New admission frames | New pending frames |
| --- | ---: | ---: | ---: | ---: |
| R4 queued | 128544 | 161696 | 35 | 35 |
| R4 claimed | 161696 | 175200 | 36 | 34 |
| R4 native committed | 175200 | 13164 | 36 | 35 |
| R5 queued | 13164 | 19440 | 35 | 35 |
| R5 claimed | 19440 | 85368 | 36 | 36 |
| R5 native committed | 85368 | 117144 | 38 | 36 |

The committed R5 ZIP is 4790 bytes with SHA-256
`8c9120053e8dd89008dbccb48800dc56e7b1bc4e9577bb6d1a6cc03fa9c99130`;
its exact bytes survive restart and all negative replay attempts. The pending
broker observations distinguish actual native effects from an external
completion receipt. No committed result is falsely reconstructed from the
old broker identity.

[The initial negative](artifacts/controller-inflight-before-2026-10-03.json)
and [diagnostic repeat](artifacts/controller-inflight-diagnostic-2026-10-03.json)
each record six passed groups before the test-side high-water failure; their
processes and credentials were cleaned and normal identity remained unchanged.
They are failure evidence, not accepted completion runs.

QA executable SHA-256 remains
`f14e39191d0836247dd62154126c9d691f92381dc7ae8523c9c780846021fd81`.
The runner proves its exact responsive maximized isolated window on every
launch, fresh per-launch authentication/process identity, no native panics,
normal-profile identity preservation and cleanup. Private ZIPs and project
files remain temporary; the repository stores hashes/observations only.

[The existing completed-terminal restart regression](artifacts/controller-inflight-restart-regression-2026-10-03.json)
passes all 19 groups with the current shared forwarding module and runner.
[Ten CLI lane exclusions](artifacts/controller-inflight-cli-2026-10-03.txt),
Node syntax, protected fingerprints and native source/evidence hashes pass.
Both positive native logs contain zero probe warnings and no native panics;
this is not a new compiler-warning measurement. The normal executable is
unchanged from the preceding `a285c1eb…` artifact and its process is outside the
QA termination boundary.

Q4 `EV-AI8-NATIVE-CONTROLLER-INFLIGHT-2026-10-03` records this bounded six-case
proof. The indexed evidence count advances from 173 to 174; no completion
status is promoted. Thirty-five denotes selected native check groups, not
Rust tests or individual assertions. The separate 19-group regression is not
added to distinct interruption coverage.

Both maintained ledger validators pass: 27 Complete, 23 Open and 8 Deferred
markers; 32 Q1 rows, 29 domains, 10 source contracts, 15 decisions, 14 risks
and 174 evidence records. The master/mirror match. All earlier Q4 records and
Q1/Q2/Q3 statuses remain unchanged. Owned diff review and all five protected
file fingerprints pass before the checkpoint commit.

## Acceptance boundary

The cached optimized isolated native executable retains the preceding pinned
MSVC 14.44.35207 build identity. This tests/docs-only checkpoint adds no compiler
warning measurement and does not rebuild or change the normal product binary.
Native evidence is specific to broker-boundary interruption of Lighting master
and diagnostic export. It does not establish interruption inside the domain
worker, durable terminal reconstruction, authored or recording crash matrices,
physical fixtures/serial DMX/video/audio, complete controller-loss/re-arm,
clean installation, venue or release acceptance. No Computer Use or subagents
are used, and protected other-owner work remains outside this checkpoint.
