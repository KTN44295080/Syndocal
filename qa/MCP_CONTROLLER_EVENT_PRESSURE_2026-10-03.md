# Native observation retention, resnapshot and broker pressure

Branch `codex/showclock-review-20260912`, base `825f7f13`.

## Reproduced defect and repair

The real authenticated MCP snapshot-to-event handoff failed with typed
`snapshot_required` before pressure began. The old optimized executable
`760ba5017a450773972d433869e23a5d8dcb203d7ca9c62be8f1fa3b059274b3`
issued the event-stream epoch as a full random `u64`. Ordinary JavaScript
Numbers could not represent that identifier exactly, so the returned fence no
longer matched the native handoff. The failure and returned fence are retained
in `artifacts/native-controller-event-pressure-negative-2026-10-03.json` and
its raw log. This was an actual native/renderer/MCP failure, not a fake broker.

The native query state now issues its event epoch through the existing CSPRNG
JavaScript-safe integer generator, like its process and session identities.
It keeps the same typed wire, strict exact-fence validation, bounded handoff,
single-use cursor, window/session owner and 60-second TTL. Opaque cursor tokens
retain 128-bit random identifiers; server-only window/cursor bindings retain
their full `u64`. An event epoch is an identifier, not an authentication secret.
No credential/grant, individual-approval, revocation, output lease or authoring
policy changes. This ephemeral process identity needs no user-data migration.

The new Rust regression checks 32 newly created query states, round-trips every
random fence identifier through JavaScript's numeric representation, and consumes
the actual issued handoff. All 15 selected query-state tests pass, none ignored.
The unit target uses test-only Engine optimization overrides; it is not the
optimized acceptance executable.

## Native test boundary

The maintained optional `--controller-event-pressure` lane runs against the
isolated optimized Windows QA executable. The independently authored fixture
project adds one stopped four-second metadata phase and a disabled A-B region;
there is no Timeline playback, media, audio, Video route, serial device or physical
fixture. Separate authenticated ExternalMcp acquire and explicit Arm start the
owned ephemeral `127.0.0.1` Art-Net route. Acquisition alone emits no packet.

The lane begins with the ordinary renderer's actual canonical MCP snapshot and
event handoff. It then uses the existing renderer-registration boundary to retain
64 immutable authenticated pending reads. A 65th request is rejected before
admission with `inflight_capacity`; its UUID remains absent. No accepted request
is silently discarded, retried as a mutation or falsely completed to free slots.

Real local fenced Loop commits alternate enabled/disabled at no more than the
existing four-per-second runtime limit. Each successful publication is captured
through the native canonical runtime observation query. This is the existing
query-driven observation stream, not proof of a separate continuous push worker
or complete per-principal fanout. The external consumer reads just one event per
64 Loop changes. Each real single-use continuation refreshes its TTL while its
backlog grows until the unchanged 2,048-event ring actually drops old records.
No synthetic clock, direct ring append, shortened retention, rate override,
invoke patch, DOM or Computer Use is used.

To read through a full broker, the lane claims one previously admitted read and
runs the production `executeAgentBridgeCanonicalOperation` against the actual
native query. Only that returned result is completed and checked through its
original MCP request UUID. The newly admitted subscriber read occupies the freed
slot, then its real result is completed and the slot refilled. All publication
iterations start with 64 pending requests. These reads use the existing trusted
renderer claim/completion seam; they do not simulate native query results or use
the output-only native executor for unsupported read commands.

The gap must be `retention_expired` in the same stream epoch with
`resnapshot_required: true`, no events and no continuation. Its first available
generation must match the real ring frontier. A cursor-expiry or process-epoch
error cannot substitute for this assertion. The subscriber then obtains actual
canonical project, output and all five runtime-domain snapshots, subscribes from
their identical fence, applies subsequent contiguous one-event pages and compares
its rebuilt state and final fence with the authoritative native snapshot.

Whole project checkpoint equality and its hash prove these runtime commits do
not alter the authored fixture, routes, metadata or project image. The receiver
checks every ArtDMX header and all 512 channels during each stable phase. Packet
interval count, mean and maximum are bounded online observations; they do not
establish the complete tick/frame/audio/render/UI or venue budget. Explicit private
empty-project replacement disarms and stops the owned sender and verifies that
the original project file remains unchanged.

## Validation results

[The final native evidence](artifacts/native-controller-event-pressure-2026-10-03.json)
and [raw log](artifacts/native-controller-event-pressure-2026-10-03.txt) pass
all 18 selected groups. QA executable SHA-256:
`26fe29b5edb25793eb7cddaf5ac753c9a0a928e867a4d70c582364fc1a5fa2ab`.

- 1,088 applied Loop changes publish 2,176 real runtime observations. The stream
  reaches generation 2,189 while the slow consumer has consumed only generation
  29. The same-epoch retention gap reports first available generation 142.
- Seventeen slow pages use cursors aged 17.60–17.68 seconds, below the real
  60-second TTL. The pressure phase takes 321.35 seconds. No native query
  Overloaded occurs in this accepted run; only terminal pre-admission read
  Overloaded permits a bounded fresh read intent, never a mutation retry.
- Maximum successful Loop publication acknowledgment is 36.53ms; maximum native
  runtime observation capture is 30.25ms on this host.
- Fresh project/output/runtime snapshots have the exact same fence at generation
  2,189. Two further Loop changes produce four contiguous deltas and the rebuilt
  client state/fence equals the actual final native state at generation 2,193.
- All 64 accepted final pending read UUIDs remain visible. Whole authored project
  checkpoint equality passes. The owned sender stops through explicit project
  replacement; original fixture bytes remain unchanged.
- 17,322 packet headers and 16,343 stable whole 512-channel payloads are checked.
  The publication phase alone checks 14,881 frames. Baseline packet interval
  mean/max is 22.65/32.25ms; pressure mean/max is 21.63/39.05ms. These observations
  are not a zero-jitter or complete realtime-budget acceptance claim.

The lane explicitly stops/restarts only its owned QA executable after retaining
the 64 pending reads, then uses a fresh principal and bridge for the seven common
lifecycle groups. It does not falsely complete pending requests for cleanup. The
runner now records actual normal-instance identity equality in failed evidence
as well as successful evidence; a mismatch still makes the gate fail. Preliminary
fixture/lifecycle runs and the run overlapping the authorized ordinary rebuild
are not accepted evidence; the final gate runs after the ordinary instance is
stable and proves it unchanged. No assertion was weakened to accept those runs.

| Gate | Result |
| --- | --- |
| Selected Rust query tests | 15 passed, 0 failed/ignored; new exact numeric/handoff case included |
| AI1 contract and self-test | PASS; two focused frontend gates and two presence cases |
| Production agent bridge contract | 13 groups passed |
| Event-pressure CLI exclusions | 13 incompatible lanes reject before executable/profile I/O |
| Exact MSVC wrapper | 249 assertions and 27 hostile fixtures passed |
| Node syntax and owned diff review | PASS |

The exact-linker optimized [QA build](artifacts/event-pressure-qa-build-2026-10-03.txt)
and ordinary `pnpm --dir app tauri build --no-bundle`
[build](artifacts/event-pressure-normal-build-2026-10-03.txt) pass. Both pin MSVC
14.44.35207 first on PATH. For both measured build configurations, Rust warnings
and TypeScript diagnostics are baseline/current/delta 0/0/0; the unchanged Vite
advisory is 1/1/0. The [selected Rust test run](artifacts/event-pressure-query-tests-2026-10-03.txt)
has zero current compiler warnings. No warning flag, limit or allowlist is changed.

[Ordinary native proof](artifacts/event-pressure-normal-window-2026-10-03.json)
pins SHA-256 `2a9ece1cf602771cb19b2303355a877f3c36a437cc642d5d1e0ba2ad80e2fd62`,
PID 95444, exactly one visible responsive maximized `Syndocal` window and rejected
unauthenticated broker access. The refreshed normal process remains running
without a debugger; this is not an authenticated primary-profile mutation.

The separate [Tap regression](artifacts/event-pressure-tap-regression-2026-10-03.json)
uses the same QA executable and the real production App callback. Its eight
native groups provide separate UI/callback evidence, not additional event-pressure
groups or a physical click test. Both accepted native runs require zero panic
locations and removal of their own QA processes and credentials. All five
protected owner-file hashes remain unchanged. These builds include pre-existing
dirty frontend work and are not clean frozen release candidates.

The Q1–Q4 mirror/authority validators preserve the 27 Complete, 23 Open and
8 Deferred markers, with the evidence index advancing from 176 to 177. No full
AI0–AI8, physical, security or release completion marker is promoted.

## Remaining acceptance

This checkpoint does not establish every event/adapter, independent security
review, Engine command-queue saturation, complete realtime budgets, cross-process
aggregate limits, native physical input, physical output, clean installation,
venue or release acceptance. It adds no subagent or Computer Use. Other-owner
frontend, PIN, localization, DJ runtime and AI3 work remains protected.
