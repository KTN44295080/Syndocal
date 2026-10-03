# MCP request capacity and authenticated native discovery

Branch `codex/showclock-review-20260912`, base `d94ac028`.

## Problem and repair

`tools/list` bypassed the per-session single-flight check. Four concurrent
requests started four descriptor/process inspections and four native requests.
[The pre-fix reproduction](artifacts/request-capacity-before-2026-10-03.txt)
exits 1 with `4 !== 1`. Only four requests were attempted before repair;
the unbounded Windows process-check path was not stressed with 10,000 requests.

Discovery now shares the session slot with tool calls. A separate process-wide
admission owner allows eight native requests across stdio, HTTP/REST and
WebSocket. It acquires the slot before descriptor/credential reads, OS process
verification and native socket creation, then retains it until owned process
verification and sockets are reaped. There is no queue, authentication cache,
automatic retry, renderer worker or output-loop change.

Overlapping discovery returns JSON-RPC `-32005`. Overlapping tool calls now
return a structured receipt with the original mutation/status UUID,
`status: rejected`, `error: sidecar_overloaded`, and an actionable `nextAction`;
the previous busy reply was plain text. This is an explicitly unsent request,
distinct from pending/unknown native execution. No public operation, grant,
HMAC, incarnation, lease, generation, fence or approval policy is retired.
Granted promoted ExternalMcp R4/R5 remains free of individual human approval.

The first real-native burst also failed: no discovery request succeeded, after
four earlier native groups passed.
[Its negative JSON](artifacts/request-capacity-burst-before-2026-10-03.json)
and [log](artifacts/request-capacity-burst-before-2026-10-03.txt) retain the failed
run and cleanup. Source inspection of `agent_bridge::process` shows successful
authentication and exact grant admission precede `ledger.begin_owned`, which
returns a pending receipt before renderer execution. The old sidecar treated
that pending receipt as an authentication failure. The negative does not store
each discovery reply; this diagnosis combines its assertion with that source.

Discovery now submits one capability read and settles only its original receipt
through at most four status lookups within a three-second settling window.
It retains both admission owners throughout. Every lookup verifies the exact
live process and current authentication. The separate ten-second process
inspection deadline still applies; an inspection already in progress is reaped
before the slot is released. Only successful completion exposes tools.
Revocation returns `-32001`; unresolved or failed authenticated discovery returns
`-32003`. Discovery is never replayed to recover a pending/unknown reply.

## Focused validation

[The maintained AI5 run](artifacts/request-capacity-ai5-2026-10-03.txt) includes:

- Existing 16 adapter groups and 128 rejected hostile stdio cases, plus existing
  HTTP/REST/WebSocket and transport-security regressions.
- Nine capacity/discovery groups: one discovery for four overlapping requests,
  bounded pending completion, revocation and failed/incomplete rejection,
  cross-method exclusion and original UUID preservation, 10,000 direct intents
  with eight admitted and 9,992 unsent, one shared budget across independent
  HTTP/REST/WebSocket sessions, slot release after descriptor failures and a
  lost mutation reply without retry.
- A throwing options proxy proves overflow does not access the descriptor,
  process or credential. The private HMAC-checking broker is a protocol fixture,
  not product-native or physical evidence.

[Eleven CLI exclusions](artifacts/request-capacity-cli-2026-10-03.txt) reject
mixed live lanes before executable selection/launch. Node syntax, AI7 source
contracts and protected-file fingerprints pass. The exact Windows build wrapper
passes 249 assertions and 27 hostile fixtures; no linker rule is weakened.

## Actual native burst

[The final native gate](artifacts/request-capacity-burst-final-2026-10-03.json)
passes all 16 selected check groups. The isolated optimized QA executable is
SHA-256 `760ba5017a450773972d433869e23a5d8dcb203d7ca9c62be8f1fa3b059274b3`.
The runner verifies one responsive maximized owned native window and uses a
separate authenticated stdio sidecar plus an owned ephemeral loopback receiver.
The project has one Lighting Art-Net route, no active Video/audio/effects and a
stopped Timeline. Acquire alone produces no packet; separate explicit Arm starts
the independently expected full 512-channel image.

| Phase | Intents | Completed | Unsent overload | Stable full-image frames |
| --- | ---: | ---: | ---: | ---: |
| Idle observation | 0 | 0 | 0 | 111 |
| Discovery burst | 10,000 | 1 | 9,999 | 54 |
| R4 master burst | 10,000 | 1 | 9,999 | 72 |

The admitted R4 no-op master request commits successfully without a dialog.
Its original-ID replay returns an identical receipt. An unsent UUID is absent
from the native ledger (`unknown`), and a fresh explicit post-burst master of
0.5 changes the received dimmer from 255 to 128 while preserving the route,
ownership and authored fixture attributes. Separate project replacement closes
output gates and stops the owned sender; the source bytes remain unchanged.
All 394 received packet headers are validated; stable phases compare every
channel, not just an in-memory preview. The run records no native read overload
or panic and cleans its process, sidecar, credentials and private files while
preserving the ordinary profile identity.

Observed receiver intervals (milliseconds) are idle median 16.46 / p95 31.92 /
maximum 32.74, discovery 16.27 / 31.59 / 32.16, and R4 master
16.41 / 31.87 / 32.07. Discovery accounting finishes in 732.50ms and R4 accounting
in 1099.73ms. These include receiver/sidecar scheduling and are observations;
they are not an invented or complete output tick/frame/audio/UI budget gate.

## Tap BPM recheck

[The current native Tap gate](artifacts/request-capacity-tap-final-2026-10-03.json)
passes eight groups against the same QA executable and final sidecar source.
The real App callback, invoked through the existing opt-in backend QA receiver,
changes 120 to 80.08954 BPM after five 750ms taps. Header `80` and footer
`Tapped BPM 80.1` match; the header stays one 42px row. A single first tap retains
the previous tempo. No second Tap implementation or product Tap behavior change
is introduced here; [the publication repair](TAP_BPM_PUBLICATION_2026-10-03.md)
owns that behavior. Nine async Tap regressions pass.

## Ordinary build and checkpoint boundary

The exact pinned MSVC 14.44.35207 QA build passes, with linker first on PATH.
[The QA build log](artifacts/request-capacity-qa-build-2026-10-03.txt) reports
6m12s for the optimized Cargo release target.
[The ordinary no-bundle build](artifacts/request-capacity-normal-build-2026-10-03.txt)
also passes and reports 2m54s for Cargo. Its wrapper stops only the verified
checkout executable, PID 175184, before replacement. A fresh plain launch has
no debugger. [The passive ordinary-window proof](artifacts/request-capacity-normal-window-2026-10-03.json)
pins PID 199664, one visible responsive maximized `Syndocal` window, and artifact
SHA-256 `b6e39dc927aa7a5184bdad9f77c14486d20a7a059089f5f74a9165e5f3b8e7a3`.
Unauthenticated broker reads are rejected. Ordinary generated assets contain
no QA Tap receiver; the normal process remains running.

For both actual builds, Rust warnings are baseline 0 / current 0 / delta 0 and
TypeScript diagnostics are 0 / 0 / 0. Vite retains its existing chunk advisory:
baseline 1 / current 1 / delta 0, verified against the preceding controller-output
builds. Probe logs contain no runtime warnings or native panic locations. No
warning threshold is changed. The builds include protected pre-existing
frontend work and are not clean frozen release candidates.

[The source manifest](artifacts/request-capacity-source-manifest-2026-10-03.json)
pins all changed production and harness modules; both final native artifacts
pin the current server and relevant harnesses. Q4
`EV-AI8-NATIVE-REQUEST-CAPACITY-2026-10-03` advances the evidence index from 174
to 175. Sixteen counts the selected native burst check groups; the nine fixture
groups, eight separate Tap groups and individual 10,000 intents are not added
to that native group count. Both ledger validators preserve 27 Complete,
23 Open and 8 Deferred markers. All earlier evidence/statuses remain unchanged.

This current-source slice does not close complete AI6/AI7/AI8, event publisher
gap/resnapshot under saturation, native internal tick/audio/video/render budgets,
local priority Blackout under saturation, multi-process aggregate limits,
complete crash durability, physical devices/fixtures, clean install, venue or
release acceptance. Each sidecar process owns its own eight slots; the native
broker remains responsible for cross-process admission. No completion marker is
promoted. No Computer Use or subagents are used; self-review is not independent
review. Protected pairing PIN, localization, DJ runtime and AI3 changes remain
outside this checkpoint.
