# Native MCP disconnect and lease TTL

Branch `codex/showclock-review-20260912`, base `b1d65784`.

## Result and behavior

The real authenticated stdio MCP/native drill passes eleven check groups. Closing
the first owned stdio process and opening a second preserves the registered native
window's lease owner and active generation. The adapter is not that owner. This
does not claim that losing or replacing the registered native owner retains its
authority; the earlier owner-retirement and restart proofs are separate.

With no renewal or synthetic clock, the backend's 60-second monotonic TTL expires.
The R0 lease query hides the expired authority; the first unavailable observation
was at `62761.0206` ms, with reads approximately five seconds apart. An expired
Renew is rejected as `forbidden` and advances the lease to orphaned generation 2.
Replaying that exact terminal request returns the same rejection without executing
again. A fresh request carrying generation 1 is also rejected and cannot advance
generation 2. Explicit Recover by the current native owner produces generation 3;
Relinquish then returns the query to unavailable.

The persisted DMX Enabled flags remain true, as in the blank project's default
configuration. Runtime ownership is startup-denied Standby, with Lighting and
Video permissions both false. The engine creates no enabled DMX sender when that
gate is closed. Configuration, master/Blackout values and both runtime gates are
compared before disconnect, after reconnect, after expiry, after recovery and
after relinquishment; they remain equal. No Enable or Arm action is issued and
the project has no Video outputs or playback.

## Query error repair

The native drill exposed a real bridge error: `query_output_lease_authority_v1`
rejects with serialized canonical QueryError, while `String(error)` collapsed its
reason to `[object Object]`. Rejected reads now keep its four bounded wire fields
in optional `result.error.native_query`, with the original `request_rejected`
classification and a useful message. Existing string/Error failures retain their
bounded message. Objects with extra fields, invalid field types or oversized
messages return a generic error rather than arbitrary exception data.

Uncertain mutations retain `mutation_not_confirmed` and never gain read retry
information. No request retry is added to the product or sidecar. The QA helpers
allow up to three new read intents only after a terminal explicitly retryable
Overloaded rejection. The native output-fence route still returns its existing
string error, while the lease route returns QueryError JSON; both current routes
are handled. Pending/unknown results and all mutations are never retried.

The passing native run records two real `overloaded` QueryError objects, each
with `retryable: true` and `resnapshot_required: false`, preserving the complete
bounded fields across the native renderer, broker and separate stdio process.

## Evidence

- [Native TTL/disconnect evidence](artifacts/native-mcp-lease-expiry-2026-10-03.json):
  eleven groups, including four lease groups and seven native crash/clean-restart,
  credential/receipt/launch-proof lifecycle groups. Every launch verifies exactly
  one responsive maximized exact-path QA window. The normal app descriptor is
  unchanged; the owned QA/stdio processes and private credentials are cleaned up.
  No native panic location is observed.
- [Native R4/R5 regression evidence](artifacts/native-mcp-lease-regression-2026-10-03.json):
  fourteen groups pass on that same QA executable, including exact grants/Safe
  Mode, diagnostic export/replay/no-overwrite, owner transfer/stale generations
  and revocation between claim and execution. Cleanup and normal identity checks
  pass; no native panic is observed. This rerun verifies the updated high-risk
  helper against the repaired read-error projection without output activation.
- Actual production bridge processor/runtime regressions: thirteen groups pass.
  New cases cover typed error retention, a single invoke without automatic retry,
  malformed/extra/oversized exception rejection and uncertain mutation rejection.
- MCP adapter integration: sixteen groups and 128 hostile inputs pass. Deferred
  bridge lifecycle: four groups pass. TypeScript and Node syntax checks pass.
- Pinned MSVC 14.44.35207 optimized isolated QA build passes. Executable SHA-256:
  `eb70fd5bc401c3a0394265d7fe1e8be9d319a91df36fe518f99da5d301b8b24a`.
  This artifact has the separate `jp.seraf.ktn.syndocal.qa.mcp-lifecycle` identity.
  No Tap QA receiver build flag is enabled for this build.

## Ordinary checkout artifact and warnings

`pnpm --dir app tauri build --no-bundle` passes with the verified absolute MSVC
linker first on PATH. The wrapper stopped only the prior exact-checkout PID
`198712` before this build. The output's `2026-10-02T21:33:36.9729907Z` timestamp
is later than the build log's `2026-10-02T21:30:34.3408506Z` creation time.
SHA-256: `65385cca3621be9a66a41993d04dc692206092757efb5e6b19eb3265a745b4e6`.
The plain launch creates PID `149208` with exactly one responsive maximized
Syndocal window. [Passive native proof](artifacts/native-mcp-lease-normal-2026-10-03.json)
pins that executable and rejects an unauthenticated read without a result.
The normal build uses no debugger or Tap QA receiver. This passive probe does
not establish authenticated native MCP or header interaction; those have their
separate isolated-native/header evidence.

[QA build log](artifacts/native-mcp-lease-qa-build-2026-10-03.txt) and
[normal build log](artifacts/native-mcp-lease-normal-build-2026-10-03.txt) retain
the exact linker preflight and optimized build completion. Current builds have
zero Rust and TypeScript warnings and one existing Vite chunk advisory. The
preceding normal optimized Tap checkpoint also measured 0/0/1: normal-build
delta is zero. No warning suppression, threshold or product version changes.
These builds include the protected pre-existing dirty frontend work and are not
a clean frozen release artifact.

Preserved negative evidence makes the harness corrections reviewable:

- [Initial precondition failure](artifacts/native-mcp-lease-expiry-precondition-2026-10-03.json)
  wrongly required persisted Enabled to be false. The replacement contract checks
  the actual closed runtime gate and unchanged configuration; it does not bypass
  sender authorization or invoke a retired configuration setter.
- [Before error projection](artifacts/native-mcp-lease-expiry-before-error-projection-2026-10-03.json)
  passed disconnect/reconnect, then failed on `request_rejected` with message
  `[object Object]`. The artifact records the failed run and cleanup; the assertion
  payload was captured by the runner's error output.
- [Before rejection assertion repair](artifacts/native-mcp-lease-expiry-before-rejection-assertion-2026-10-03.json)
  reached expired Renew, whose correct domain rejection has `ok: false`. The
  helper initially required success even for this expected rejection. It now
  requires false for these two rejection cases and true for all successful
  mutations, then asserts the exact domain error, replay and generation outcome.

## Acceptance boundary

This is native software authority and configuration preservation evidence.
It is not a physical signal, complete registered-controller-loss/re-arm, durable
authored publication, clean installation, signing or release acceptance. AI8 and
`R-AI-SAFETY-001` remain Open. The user-authorized ExternalMcp all-R4/R5 policy
still uses exact grants without individual human approval.

Protected Remote PIN/DJ ingress work is excluded. No subagents or Computer Use
were used; review was by the implementing agent.

The master and machine Q1-Q4 mirrors index the bounded native evidence as record
165. Status remains 27 Complete, 23 Open, 8 Deferred. Completion and Q1-Q4 checks
pass, including exact mirror equality; the index validation caught and repaired
a temporary string-replacement error before staging the final master diff.
`git diff --check` passes. The next bounded goal work is the stale risk-description
audit for ShowClock/migration/patch, followed by executable missing corpus or
native controller-owner drills; no physical acceptance follows from this slice.
