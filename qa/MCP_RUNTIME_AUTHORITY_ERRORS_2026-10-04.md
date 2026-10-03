# Native Timeline authority errors through MCP

Branch `codex/showclock-review-20260912`, base `42f7f1e8`.

## Confirmed failures

The three Timeline authority readers converted every project-fence capture
failure to `forbidden`. The canonical capture explicitly returns `overloaded`
for coordinator contention and `internal` for poison; neither is permission
denial. Exhausted bounded source-capture attempts return `unavailable`.

The renderer's MCP adapter recognized canonical four-field `QueryError`, but
discarded the separate one-field `RuntimeCommandErrorV1`. The real isolated
Windows [negative run](artifacts/runtime-authority-native-before-2026-10-04.json)
observed native idle Follow `{code:"stale_fence"}`. The actual authenticated
stdio request completed with `ok:false` and a generic unrecognized-error message,
without the native code. Its old executable SHA-256 is
`26fe29b5edb25793eb7cddaf5ac753c9a0a928e867a4d70c582364fc1a5fa2ab`.
The normal process was unchanged, own credentials/processes were removed, and
no native panic was recorded. Its source hash fields describe the candidate
working tree at observation time, not the old executable's compiled source.

Preliminary probe setup exposed the intentionally unavailable Follow capability
on a fresh process. The final probe explicitly tests that rejection, then uses
the ordinary New operation on its private empty project to establish a Follow
generation. A preliminary pressure attempt used synchronous bundle reads and
observed no contention; it is not contention proof. The final lane requires
asynchronous checkpoint reads and at least one actual `overloaded` response.

The [first rebuilt pressure trial](artifacts/runtime-authority-native-pressure-before-2026-10-04.json)
confirmed 50 actual authority `overloaded`
responses, but rejected the probe's assumption that all 2,048 companion recovery
captures must succeed. Production `get_project_checkpoint_bundle_core` explicitly
uses admission/coordinator `try_lock` and returns `Project recovery capture is
busy; retry` under contention. The final probe accounts for every offered call:
successful captures must carry the exact unchanged E/R/H, only that specific busy
rejection is accepted, and at least one capture must succeed. Unknown failures,
missing results, authority `forbidden`/`internal`, absent real contention and
authored/runtime/output changes remain failures. Product admission and rejection
policy is unchanged. The unsuccessful trial is not accepted native evidence.

## Changed behavior

All three production readers share one project-fence capture/error boundary.
True unauthorized/forbidden failures stay `forbidden`. Rate/capacity/contention
failures become `overloaded`; a source without a stable bounded capture becomes
`busy` because the established runtime V1 has no `unavailable` code. Stale
snapshots become `stale_fence`, malformed requests remain `invalid_request`,
and internal/unsupported service states remain `internal`. Native messages,
paths and exception data do not cross this fixed-code DTO.

The MCP adapter exposes the exact eight-code `RuntimeCommandErrorV1` as
`error.native_runtime` only on the three reviewed authority reads. Extra fields,
unknown codes and other operations do not match this decoder. The outer error
remains `request_rejected`; uncertain mutations remain `mutation_not_confirmed`
and receive no read error metadata. The adapter never retries a read or mutation.
This is an additive MCP failure-detail field, with no project or wire-version
migration. Authentication, exact grants, revocation, immutable request binding,
owner/fence validation, lease behavior, TTL and rate limits remain intact.

## Software verification

[The selected Rust module](artifacts/runtime-authority-tests-2026-10-04.txt)
passes **35 tests, 0 failed, 0 ignored**, including four new regressions:
fixed-code/redacted mapping, held-coordinator contention and subsequent recovery,
unregistered-owner rejection even during contention, and poisoned-coordinator
classification. The contention test calls the same production capture helper
used by all three readers and requires it to return within 250ms.

The test command uses the maintained exact MSVC 14.44.35207 wrapper environment,
prints the pinned absolute linker and PATH-first result, and uses test-only
Engine optimization overrides (`opt-level=0`, `codegen-units=256`). It does not
produce the native acceptance executable. The selected test configuration has
zero current compiler warnings.

[Frontend contracts](artifacts/runtime-authority-contracts-2026-10-04.txt) pass:
the production bridge runs 15 groups including all three reads/eight runtime
codes, hostile DTO redaction, unrelated-read exclusion and mutation non-retry;
the existing Loop, transport and Follow contracts also pass.
[Sixteen CLI exclusions](artifacts/runtime-authority-cli-2026-10-04.json) reject
incompatible optional lanes before executable/profile I/O. Node syntax passes.

## Native acceptance and remaining boundary

[The optimized QA build](artifacts/runtime-authority-qa-build-2026-10-04.txt)
passes with the exact MSVC 14.44.35207 linker pinned and first on PATH. Its
executable SHA-256 is
`23885b3ea23aee245e26429b5309e3501fabe02d2b116b07c87ed3a2f02faf97`.

[The final native lane](artifacts/runtime-authority-native-final-2026-10-04.json)
passes **12 groups**: five authority-specific groups plus the seven maintained
authentication/restart/lifecycle groups. All three real authority reads succeed
through native IPC and authenticated stdio MCP under exact read grants and Safe
Mode; the previously lost idle Follow `stale_fence` now survives the actual MCP
response. The native pressure phase observes:

| Observation | Count |
| --- | ---: |
| Rounds | 64 |
| Native authority attempts | 192 |
| Successful authority reads | 154 |
| Actual typed `overloaded` authority responses | 38 |
| Offered asynchronous checkpoint reads | 2,048 |
| Successful captures with exact unchanged E/R/H | 1,160 |
| Explicit production recovery-capture busy rejections | 888 |

Every call has an actual result. No error classification, lock, response, engine
queue or invoke function is patched. Fresh authenticated MCP reads for all three
domains succeed after pressure. The whole authored project checkpoint, Timeline
runtime and output ownership equal their pre-pressure values; both output gates
remain closed. The private fixture setup uses the ordinary New operation, not a
synthetic runtime generation. The final accepted run pins the current runner,
native source, renderer adapter and harness hashes. Own QA processes/credentials
are removed, normal-profile identity is unchanged, and no native panic is found.

[The separate Tap regression](artifacts/runtime-authority-tap-2026-10-04.json)
passes eight native groups on the same QA executable. Five 750ms target taps run
the real App callback, change the engine from 120 to `81.42337` BPM, and finish
with header `81` and 42px height. The long-pause reset still preserves BPM on the
first tap. This is callback/header evidence, not physical-click acceptance and
not eight additional authority-pressure groups.

[The ordinary `pnpm --dir app tauri build --no-bundle` build](artifacts/runtime-authority-normal-build-2026-10-04.txt)
passes. It resolves/verifies this checkout's exact executable immediately before
building and the maintained wrapper stops only its prior PID 95444. Both measured
optimized build configurations pin MSVC 14.44.35207 first on PATH. Relative to
the previous corresponding build logs, Rust warnings and TypeScript diagnostics
are baseline/current/delta **0/0/0**; the existing Vite advisory is **1/1/0**.
No warning flag, chunk limit or allowlist is changed.

[The refreshed ordinary window proof](artifacts/runtime-authority-normal-window-2026-10-04.json)
pins SHA-256
`f80b9c361b6f270a56cb104e03156bd284d168c922638d72eba19c93fa110c0a`,
PID 135288 and exactly one visible responsive maximized `Syndocal` window.
Plain launch enables no debugger; the probe waits for the descriptor to bind to
this actual new process and verifies unauthenticated broker rejection. The normal
process remains running. This is passive window/authentication evidence, not
authenticated primary-profile mutation or geometry/physical-click acceptance.

The Q1–Q4 update adds this bounded proof and preserves all completion-marker and
requirement statuses: 27 Complete, 23 Open and 8 Deferred, with Q4 advancing from
177 to 178 evidence rows. Independent review is not performed under the user's
no-subagent instruction; the stable owned diff and actual evidence are reviewed
by the supervising agent. The next acceptance boundary remains the broader
adversarial/adapter/runtime and external matrices recorded in the AI roadmap.

This focused repair does not close full AI0–AI8, every adapter/event, complete
lock-wait or realtime budgets, independent security review, hardware, clean
installation, venue, support/update, or release acceptance. It performs no
physical output activation, Computer Use or subagent work. The five protected
other-owner files remain unchanged; local builds include that pre-existing dirty
frontend work and are not frozen clean release candidates.
