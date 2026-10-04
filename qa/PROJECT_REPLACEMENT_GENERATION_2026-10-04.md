# New/Open publication generation at replacement boundaries

Branch `codex/showclock-review-20260912`, base `a2f8855e`.
This extends [project replacement preflight](PROJECT_REPLACEMENT_PREFLIGHT_2026-10-04.md)
as a prerequisite for typed external New/Open. Those MCP operations are still
unimplemented; the full environment-executable completion goal remains active.

## Failure, change and compatibility

The [previous native executable](artifacts/replacement-generation-before-2026-10-04.json),
SHA-256 `bd7e1f05dc95bbd1be91cc06d297c77b5808d55cac43db5fa1a26d4312d438f1`,
accepted New with requested publication generation 3 while the current
generation was 2. It advanced epoch 1 to 2 and closed both output gates instead
of rejecting. The old Tauri signature ignored the additional argument.
The failure and cleanup are retained; the ordinary instance stayed unchanged
and no native worker panic occurred. Source hashes from this negative run
identify files present during observation, not the old executable's build.

New and path-based Open now accept `expectedPublicationGeneration`. When
supplied, its JavaScript-safe integer value is retained in the immutable
invocation fence and checked at initial/lifecycle preflight and the final
coordinator-held publication validation. Known stale requests reject before
managed output retirement. A changed publication generation also rejects when
E/R/H are unchanged; owner-incarnation revalidation remains enforced.

The argument is optional for existing local callers, whose E/R/H contract
continues unchanged. Current native evidence covers successful supplied
generation and existing omitted-argument paths. This is an additive local
Tauri argument, not a permissive fallback in a new canonical MCP schema. The
future typed external request must require the full issued fence. Tauri names
and admission inventory remain 542; canonical operations remain 53. No
operation, consent rule, grant, recovery schema or product version was added.

## Current evidence

- [Native replacement probe](artifacts/replacement-generation-native-2026-10-04.json):
  22 groups pass (15 focused plus seven maintained lifecycle groups). All eight
  New/Open E/R/H/publication-generation rejections preserve complete authority
  metadata, checkpoint and gates, with 23-24 newly received complete live
  512-channel frames per rejection. Valid Open/New still deliver zero and stop
  the sender. Explicit Enable keeps S0; explicit Release restores the image.
- [Rust tests](artifacts/replacement-generation-tests-2026-10-04.txt):
  six new generation tests and the complete 12-test replacement filter pass.
  Six tests overlap, so this is 12 unique tests, zero failed/ignored. The new
  cases cover initial stale rejection, same-E/R/H publication interleaving,
  exact validation, unsafe integer, owner-incarnation rotation and legacy
  omission. Interleavings are simulated against the actual validator, not
  claimed as observed simultaneous native mutations. Recorded test-only
  optimization overrides do not stand in for the application builds.
- [Native output regression](artifacts/replacement-generation-output-regression-2026-10-04.json):
  16 groups pass, including existing omitted-generation load paths.
  [Tap regression](artifacts/replacement-generation-tap-2026-10-04.json):
  eight groups pass, engine BPM 120 to 79.82331, header 80, height 42px and
  long-pause reset. The existing registered App callback is invoked by its
  opt-in QA receiver; no physical click or full Control layout audit is claimed.
- [Normal build](artifacts/replacement-generation-normal-build-2026-10-04.txt)
  and [sequential QA build](artifacts/replacement-generation-qa-build-2026-10-04.txt)
  pass the maintained no-bundle and pinned PATH-first MSVC 14.44.35207 gate.
  Normal SHA-256:
  `c7280106e8d111c8f9f2e07554d175b84e8d5acb39d3707efb50aa53d99f9702`.
  QA SHA-256:
  `a2d3e1e216fda23df89fed8e23e4ef8f679dcb9f8f47d0abe833e13e6c1f2fd6`.
  All three current native probes use that QA hash, preserve ordinary identity,
  report zero native panics and clean up their process/credentials.
- [Ordinary window after probes](artifacts/replacement-generation-normal-window-2026-10-04.json):
  exact normal executable at PID 184464 has one visible responsive maximized
  `Syndocal` window; unauthenticated read rejects. No primary debugger or
  authenticated primary-profile mutation was used.
- [Source freeze](artifacts/replacement-generation-source-freeze-2026-10-04.json):
  product/Rust source stayed identical through tests and builds. Before current
  probes the harness added the separate exact publication-generation error
  assertion; the final harness freeze records that adjustment and stays
  unchanged through the probes. Five protected other-owner fingerprints remain
  unchanged. Owned diff, harness syntax, project-authority contract, exact
  admission inventory (18 negatives) and both ledger checks pass.

Rust warning/TypeScript diagnostic baseline/current/delta is 0/0/0; the existing
Vite size advisory is 1/1/0. Node-only contracts have no compiler-warning
measurement. No allowance or size threshold changed. User restrictions on
subagents and Computer Use were honored; stable diff/evidence inspection by
the primary agent does not constitute independent review.

Known-stale rejection is proved. Output preservation if authority changes after
initial preflight during retirement remains unproved; final publication still
rejects. Complete concurrent replacement, physical/serial/video/venue/release
and wider domain acceptance remain open. Next implement typed canonical New
and then Open using the existing replacement primitive, server-derived current
main-owner incarnation, required issued process/session/E/R/H/publication fence,
immutable authenticated request, exact File grant/revocation and terminal
receipts/replay. Do not expose raw New/Open passthroughs. Q4 adds bounded
evidence without changing the 27 Complete / 23 Open / eight Deferred markers.
