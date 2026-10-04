# Project replacement preflight and managed network retirement

Branch `codex/showclock-review-20260912`, base `6c76fae6`.
Bounded AI3 project-replacement and AI8 native backend evidence; the full
completion goal remains active.

## Observed failure and resulting behavior

The [baseline native failure](artifacts/project-replacement-negative-2026-10-04.json)
used executable SHA-256
`180f103cdc44a7b0cf09ba1f64237567f917337cdcf9cb967832d177aa84ad1d`.
A New request with an already-stale project epoch rejected, retained E/R/H,
but changed both output gates from allowed to denied. Output retirement ran
before replacement authority validation. The shared default and platform-backed
replacement paths now validate authority before that side effect, release
their short-lived locks, and retain the existing lifecycle preflight and final
publication CAS. Known stale epoch, revision and checkpoint-hash requests for
both New and Open preserve the full checkpoint, authority metadata, output
gates and continuous complete 512-channel image.

The [partial native candidate](artifacts/project-replacement-partial-2026-10-04.json)
passed all six stale-request cases, but a valid Open still failed because
managed retirement used the exact show-only Art-Net/USB topology. The generic
managed network path now checks exact S0/failure authority and retires all
configured Art-Net/sACN routes. It sends one full zero datagram per enabled
route, drops every network sender and its reconnect state, and preserves
configuration. Send failures continue retirement of other routes and return
InDoubt while retaining S0 and the failure fence. Cancellation and known stale
authority reject before sends. Existing verified show USB workers/history
continue through the unchanged strict show path; general serial retirement
remains unsupported and fails closed.

The [zero-send negative candidate](artifacts/project-replacement-zero-negative-2026-10-04.json)
revealed that the earlier S0/failure publication could already have removed
the persistent sender. That case now creates one local zero-only transport to
the current authoritative network configuration and drops it within the
consumed retirement operation. It never installs a persistent sender, arms,
retries or sends a live frame. Disabled absent routes emit nothing. The generic
API discards the old internal result and never exposes a fabricated exact-show
datagram/USB receipt. No public operation or serialization schema was added.

## Current evidence

- [Optimized normal build](artifacts/project-replacement-normal-build-2026-10-04.txt)
  and [optimized QA build](artifacts/project-replacement-qa-build-2026-10-04.txt)
  use the maintained no-bundle wrapper and pinned PATH-first MSVC 14.44.35207.
  Normal SHA-256:
  `954d8d828bf1fa5a8ab8e542ad5a1e5a90dddaf180bef6bddefdaa1c40baa970`.
  Final isolated QA SHA-256:
  `bd7e1f05dc95bbd1be91cc06d297c77b5808d55cac43db5fa1a26d4312d438f1`.
- [Native preflight probe](artifacts/project-replacement-native-2026-10-04.json):
  20 groups pass, comprising 13 replacement/output/source-preservation groups
  and seven maintained authentication/crash/graceful lifecycle groups. The
  six stale requests each retain at least ten new complete live frames.
  Valid Open/New advance the project epoch, close both gates, deliver a full
  zero image and stop the sender. Open retains S0; Enable alone produces live
  zero frames. Separate explicit ReleaseBlackout restores the authored image.
  Source-file bytes, normal identity and owned cleanup are verified.
- [Existing native output regression](artifacts/project-replacement-output-regression-2026-10-04.json):
  16 groups pass. [Tap regression](artifacts/project-replacement-tap-2026-10-04.json):
  eight groups pass. Five 750ms taps change engine BPM 120 to 79.67188, header
  to 80 and footer to `Tapped BPM 79.7`; the single-row header stays 42px.
  A long pause resets Tap history without changing tempo on its first tap.
  Both regressions use the exact final QA executable, with zero native panics
  and owned process/credential cleanup. Tap invokes the real registered App
  callback through the existing opt-in QA receiver; no physical click is claimed.
- [Rust regressions](artifacts/project-replacement-tests-2026-10-04.txt):
  47 passed, zero failed/ignored: seven configured-network tests, 23 existing
  strict-show fail-stop tests, six native project-replacement tests and 11
  managed keepalive integration tests. Engine/Syndocal test-only optimization
  overrides are recorded; the two application builds above use normal
  optimized release profiles. The network tests use owned software sockets.
- [CLI rejection contract](artifacts/project-replacement-cli-2026-10-04.json):
  all 19 other probe flags reject in both orders before native/profile access,
  38 cases. Admission inventory remains 542 commands with 18 negative cases;
  canonical registry remains 53 operations. Output-control/runtime ownership,
  agent bridge (15 groups), maintained build-wrapper (249 assertions/27
  hostile fixtures), harness syntax and owned diff checks pass.
- [Passive ordinary window proof after the probes](artifacts/project-replacement-normal-window-2026-10-04.json)
  verifies exactly one visible responsive maximized `Syndocal` window at PID
  116744 and unauthenticated-read rejection. No primary debugger or
  authenticated primary-profile mutation was used.
- [Source freeze](artifacts/project-replacement-source-freeze-2026-10-04.json)
  is unchanged through the tests/builds/probes. All five protected other-owner
  fingerprints are unchanged, including App.tsx. Rust warnings/TypeScript
  diagnostics baseline/current/delta 0/0/0; existing Vite advisory 1/1/0.
  Node-only contracts have no compiler-warning measurement.

An intermediate Tap run [failed before Tap mutation](artifacts/project-replacement-tap-hook-negative-2026-10-04.json)
because simultaneous normal/QA Vite builds shared `app/dist` and embedded the
normal frontend into the QA executable. The final QA build ran sequentially
and all three native probes passed on its exact hash. The shared-dist boundary
is now recorded in [Windows build guidance](WINDOWS_NATIVE_BUILD.md).
Earlier probe development also corrected comparison of free-running snapshot
clock values, the explicit Enable/Release S0 contract, and a runner count
expectation of five native tests when six actually ran; no product assertion
was weakened or failing product test counted as a pass. One
[fresh probe received an initial unknown read](artifacts/project-replacement-initial-read-negative-2026-10-04.json)
and failed; it is not a startup-matrix pass.

## Remaining boundary and next action

This proves rejection of requests already stale at the initial preflight,
not atomic output preservation if authority changes during retirement/stop.
The final publication CAS still rejects such a change. General serial and
USB hardware, physical fixture/video effects, network-hostname cold-I/O and
complete realtime budgets, concurrent replacement, full recovery/backup/Take
Over, independent review, venue and release acceptance remain open. The user
required no subagents and no Computer Use; the stable diff and evidence were
inspected by the primary agent, with no independent agent-review claim.

New/Open here use the existing native backend with a registered QA owner;
only Enable/ReleaseBlackout use authenticated external stdio MCP. Typed
external New/Open is still absent. Next implement canonical typed New with
server-derived current main-owner incarnation, immutable mutation identity,
issued publication-generation fence, final CAS, exact grants/revocation and
typed terminal/replay receipts; do not expose a raw Tauri passthrough. Open,
Save and recording capability gaps and the wider goal remain open. Q4 gains
bounded evidence while the 27 Complete / 23 Open / eight Deferred requirement
counts and open safety risk remain unchanged.
