# Native Timeline commands and revocation

Branch `codex/showclock-review-20260912`, base `c8d141e60d6b5acccc5e2a9108c06bd1238be146`.

External MCP Play/Pause, Loop commit and Follow Abort now execute the immutable
authenticated native broker request. The renderer forwards only its generation
and UUID; retired direct frontend invocation refuses these three operations.
The native adapter decodes the original typed body, checks operation identity,
and shares the existing GUI domain implementation. Only a typed Receipt is
success; typed Rejected results remain `ok:false`.

The shared domain checks the exact current external grant at entry, after owner
binding, and after admission/coordinator waits before capability consumption and
Engine publication. Existing local owner/session, project/runtime fences, TTL,
rate, lane and terminal receipt rules remain. Revocation does not grant retry.
No public command, protocol schema, version, UI size or external approval policy
changes. R4/R5 approval supersession and authentication remain as previously
authorized. The focused adapter adds no frame copies or background workers.

## Evidence

- Final selected native runtime units: 39 passed, zero failed/ignored. Bridge
  selection: 29 passed, zero failed/ignored; two guard tests overlap both counts.
  Real disarmed Engine tests deny the final Transport/Loop authorization after
  both earlier checks, preserve runtime/output and retain the denied terminal
  on replay. This injected domain callback is not an AgentAuthority race test.
- Static bridge: 16 groups; native admission: 561 exact commands and 18 rejected
  fixtures. Tap controller: nine async cases passed again without source changes.
- Exact MSVC 14.44.35207 pinned/PATH-first optimized QA and ordinary
  `pnpm --dir app tauri build --no-bundle` both passed. Rust/TypeScript warning
  baseline/current/delta: 0/0/0. Existing Vite chunk advisory: 1/1/0.
- [Actual native stdio gate](artifacts/native-timeline-guard-native-saved-2026-10-07.json): 21 checks passed. Three
  exact UUID replays preserve the runtime; forged authority fences refuse with
  canonical Transport/Loop Forbidden and Follow StaleFence categories. Empty
  Loop enable reports PublicationFailed as false. Actual Play publishes Applied
  with output disarmed; unmodified authorities from before that successor refuse
  Transport/Loop as StaleFence. Authored checkpoint and output are unchanged.
- After claim, each of the three real external principals is revoked. Native
  execute returns agent_principal_revoked, replay returns request_not_executable,
  and no result/state change is exposed. Ordinary native close and fresh restart
  pass; retired grants, credentials, receipts and launch proofs are rejected.
  QA credentials/processes and the exclusively owned temporary saved project
  are cleaned up. The normal instance descriptor remained unchanged throughout.
- [Ordinary window proof](artifacts/native-timeline-guard-normal-window-2026-10-07.json): the previous exact PID 148908
  was closed gracefully, no forced primary termination; exactly one visible,
  responsive, maximized Syndocal window is verified with unauthenticated reads
  still rejected. Current executable SHA-256: `0abce29e600c60b6e60202d3ef74aed10a68f3ba0df799d0337483c507330ab8`.

Private executable SHA-256: `14559e0d6533db0d82bcb416efa405e51341a1a56d88c29392b1a4134aefe8d0`. Rust/frontend sources
match its build freeze. Only the two nonembedded native harness files changed
after that build to correct fixtures/close preparation; the successful native
artifact and final normal build freeze record their actual final hashes. The
preliminary two-test unit run predates only poison-error categorization; final
39/29 selections include both tests. Complete successful harness elapsed time
is approximately 33.3 seconds (startup, IPC and restarts;
not Engine tick, audio latency or physical performance measurement).

## Retained failed evidence and boundaries

All raw runs remain immutable in Temp; repository copies normalize only trailing
whitespace. The initial new-profile refusal ran no app. Two failed fixtures
mistakenly expected the same refusal category across differently ordered domain
checks; assertions now require each exact canonical category. The later run
passed 18 checks but failed ordinary close because New left an unsaved project;
this is retained as failed overall evidence. The next run used a noncanonical
Windows path spelling in its test assertion. Final preparation uses canonical
File SaveAs plus Acknowledge, validates typed phases, canonical path and file
digest, then retires the renderer; close protection is never bypassed. Original
positive/refusal assertions and lifecycle checks remain enforced.

Typed frontend assertions changed intentionally: retired direct invocation now
must make zero calls, and native typed refusals must remain false. No assertions
were removed merely to turn a failed product behavior green. Stable self-review
is not independent review. Protected five files and unrelated .vite content
remain unowned/unstaged and retain their hashes.

This is bounded Timeline native admission/receipt/lifecycle evidence. It does
not establish cancellation after an already admitted Engine command, a direct
Follow wait-race test, active Follow cancellation, Loop with actual A-B bounds,
all R2/R3 or every adapter, durable effect audit, physical Control Tap clicks,
the primary operator clock source, fixtures/video/venue or full AI8/Timeline/
release acceptance. Remaining authored and legacy fixture native routing and
durable audit stay open; the broader goal remains active. Requirement markers
remain 27 Complete /23 Open /8 Deferred. Q4 evidence advances 198 to 199.
