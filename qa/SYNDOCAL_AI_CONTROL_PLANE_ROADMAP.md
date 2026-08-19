# Syndocal AI Control Plane roadmap

## 1. Product objective

Syndocal must be fully operable by an AI or another automation client through
backend operations. This does not mean automating the DOM. Every operation that
can affect a project, show runtime, device, file, recording, output, or safety
state must have a typed backend command or query. The desktop UI, shortcuts,
MIDI/OSC, Remote WebSocket, MCP, and external API must converge on those same
domain operations.

Pure presentation state may remain frontend-local: panel disclosure, focus,
selection rectangle, zoom, scroll, and window placement. If a GUI action changes
authored or runtime show truth, it is not presentation state and must be present
in the control plane inventory.

The completion invariant is:

> Every non-presentational GUI action maps to exactly one registered backend
> operation, and every registered operation declares its state class, risk class,
> capability, schema, idempotency, receipt, audit, and availability contract.

MCP and the external API are adapters over that registry. They are not a second
implementation of Syndocal behavior.

## 2. Canonical architecture

### 2.1 Typed Command and Query Registry

The backend owns one generated, versioned registry. Each entry includes:

- stable operation ID and contract version;
- Command or Query classification;
- authored, runtime, output, file, administrative, or presentation classification;
- input, result, terminal receipt, event, and typed-error schemas;
- required capability and risk class;
- idempotency and concurrency semantics;
- consent requirement and target-resource fingerprint;
- Undo/history behavior;
- rate and payload limits;
- supported adapters and availability reason.

Unregistered backend mutations and adapter-specific domain logic are forbidden.
An unclassified new command is denied as highest risk until explicitly reviewed.
Rust request/result/error types are the source of truth. JSON Schema, MCP tools,
OpenAPI/JSON-RPC discovery, TypeScript bindings, and conformance fixtures are
generated projections; hand-maintained competing schemas are not accepted.

### 2.2 State classes

- A Query is side-effect free and runs under a read capability.
- An authored Command carries `request_id`, expected project epoch/revision,
  checkpoint hash, principal/owner incarnation, and shape fingerprint. It commits
  atomically and publishes exactly one Undo/history entry.
- A runtime Command changes ephemeral transport/live truth, has a monotonically
  ordered runtime generation where needed, never reshapes persistence, and never
  enters Undo/history.
- An output Command is additionally fenced by current output ownership and
  physical/live safety state.
- A file/recording Command is terminal and idempotent by request and target
  resource; retry cannot create a second file or writer.

The registry rejects cross-class requests rather than coercing them. Adapters do
not invent request IDs, fill current epochs, retry mutations, or reinterpret an
error. Reply loss is recovered by exact request ID and terminal receipt lookup.

### 2.3 Adapters

- Tauri is the trusted local desktop adapter.
- Existing MIDI, OSC, keyboard, and Remote WebSocket routes migrate to registry
  operations without changing their supported behavior accidentally.
- MCP runs in a separate optional localhost-only sidecar. Loopback binding is
  isolation, not authentication. A sidecar crash or restart cannot stop,
  restore, Blackout, release, or otherwise actuate outputs.
- JSON-RPC 2.0 is the canonical external request/response wire format. A REST
  facade may be generated for conventional integrations. WebSocket carries the
  same Commands/Queries plus typed subscriptions; it is not an event-as-command
  bypass.
- LAN exposure is off by default and requires an explicit secure configuration.
  The localhost sidecar remains a separate principal from the desktop UI.

The sidecar holds no project authority, authored cache, capability policy, or
consent policy. Those remain in the main backend registry. It has bounded queues,
bounded messages, bounded concurrency, and a dedicated kill target so model I/O
cannot disturb the realtime engine or 44 Hz output budget.

### 2.4 Local authentication and principal binding

The main process and sidecar authenticate over a per-launch IPC channel restricted
to the current Windows user/SID. The main process creates an unguessable ephemeral
session secret and transfers it through an inherited anonymous pipe or equivalent
non-command-line, non-environment, non-log transport. The sidecar proves possession
before registry discovery or forwarding is enabled. A TCP port, process name, PID,
or localhost source address is never accepted as identity.

Each MCP/API client is a distinct principal. Pairing starts from the Syndocal
desktop, produces a single-use, expiring pairing challenge, and installs a
per-principal credential protected by the OS credential store/DPAPI. Connections
use nonce-based challenge-response and receive a short-lived connection token
bound to adapter, principal, sidecar session, grants, and expiry. Long-lived
credentials and session tokens are revocable independently; neither appears in
URLs, command lines, environment variables, logs, diagnostics, or project files.
Unpaired clients may access only the unauthenticated health/version compatibility
probe; registry discovery, snapshots, resources, tools, events, and all Commands
require an authenticated principal.

## 3. Risk, capability, and consent model

Every operation declares one risk class:

| Class | Typical operations | Minimum policy |
| --- | --- | --- |
| `R0 Read` | discovery, project/status/telemetry queries | scoped read grant |
| `R1 ReversibleRuntime` | select, preview, fader, seek/nudge | runtime grant, rate limit, audit |
| `R2 LiveVisible` | GO, Take, play, submaster, cue activation | live grant, tighter rate limit, audit |
| `R3 AuthoredMutation` | Cue/Timeline/Patch/media/mapping edits | authored grant, E/R/H, one Undo, receipt |
| `R4 OutputDisruptive` | Blackout release, output Arm/Takeover, device switch | output grant, ownership, human-present consent |
| `R5 FileOrReplacement` | open/new/replace, recording, overwrite/export | explicit grant, prepared single-use consent |
| `S0 SafetyOnly` | `SafetyBlackoutEngage`, current-owner `ResourceSafeStop` | narrow named capability, safer-direction-only implementation, no per-action consent, priority queue, strict rate limit, immutable audit |

Capabilities are scoped per principal, adapter, risk class, optional project, and
optional operation family. There is no bearer token that silently means all
operations. Newly paired external principals start in safe mode: `R0` and the
explicitly allowed subset of `R1`; `R3`-`R5` are denied until promoted locally.

`R4` and `R5` require a backend-issued short-lived, single-use confirmation token
bound to principal, owner incarnation, operation ID, canonical argument
fingerprint, project/output generation, and expiry. The Syndocal desktop displays
and accepts consent; an MCP client cannot approve itself. Blackout engagement may
remain an emergency fail-safe path, but Blackout release, output Arm/Takeover, and
other energizing operations never inherit blanket consent.

Emergency Blackout engagement is a separate `SafetyBlackoutEngage` operation, not
the ordinary `R4` Blackout-state setter. The local desktop always retains its
priority path. An external principal may invoke it without per-action consent only
when the operator has explicitly granted the narrow
`safety.blackout.engage` capability to that principal; it is not included in new
principal safe mode, `R4`, or an administrator wildcard. It can only transition
toward the safer Blackout-on state, is idempotent, priority-queued, rate limited,
and fully audited. Blackout release is always the ordinary energizing `R4` path
with fresh local consent. No argument or adapter can turn the emergency operation
into a toggle or release.

`S0` is not a general capability and is not ordered above `R5`. The registry
accepts only operations whose implementation is statically proven to move one
named resource toward its safe terminal state and cannot accept a target value.
`SafetyBlackoutEngage` requires only `safety.blackout.engage`, permits at most four
attempts per second with a burst of eight, and deduplicates the already-on state.
`ResourceSafeStop` requires `recording.safe_stop`, is limited to one accepted
finalization per recording session, and cannot open or replace a sink. Both are
audited at the same level as `R4`/`R5`. The inventory/coverage gate treats `S0` as
an explicit risk class and fails any unreviewed addition to it.

Human presence is a backend state, not an MCP assertion or ordinary DOM click.
For `R4`/`R5`, the desktop displays a fresh random confirmation challenge and the
backend accepts it only after matching `WM_INPUT`/Raw Input from an enumerated
physical keyboard, mouse, or approved operator HID. `SendInput`, posted/synthetic
window messages, accessibility invocation, UI automation, Remote input, sidecar
traffic, API events, a previously issued token, and mere process/window existence
cannot mint or renew presence. Standard injected-input flags are denied as defense
in depth but are not the sole proof. The verified physical challenge starts a
presence lease of at most 30 seconds; the resulting per-action token expires in at
most 15 seconds and consumes the lease. Desktop lock, Standby, project replacement,
principal revocation, device removal, and expiry invalidate outstanding presence
and confirmation tokens. Any test-only bypass is compile-time excluded from release
builds and its absence is checked in the release artifact.

The operator has a priority kill switch that immediately revokes all non-UI
principals and rejects pending external commands without blocking local Blackout
or Standby controls. Controller or sidecar loss leaves current engine-owned output
unchanged; it never automatically energizes, restores, releases, or Blackouts.

Output ownership uses explicit backend leases. Acquisition and forced transfer are
`R4` actions with current resource generation and local consent. A lease has an
owner incarnation, resource set, TTL, and monotonically increasing generation;
only the current owner may renew it, and retry is idempotent by request ID. Expiry,
disconnect, kill-switch, or missed heartbeat revokes future command authority but
does not change physical output. The resource becomes `held_orphaned` until the
verified local UI renews the same owner, performs a fenced forced transfer, moves
the resource to Standby/Blackout, or releases it. The local UI is subject to the
same output fence for normal commands while retaining priority Blackout and the
explicit recovery/transfer workflow. A sidecar cannot reclaim or transfer a lease
on restart from cached state.

Recording start, sink/path selection, overwrite, and replacement are `R5`.
Idempotent stop/finalize by the current recording owner is a distinct
`S0 ResourceSafeStop` operation that remains available during safe mode and does not
require a new destructive consent token; it may only close the current session and
cannot select, overwrite, or start a sink. A different principal needs an explicit
locally approved takeover before it may finalize. Emergency disk-full/device-loss
finalization is backend-owned and audited rather than blocked on an AI client.

## 4. MCP and API product surface

### 4.1 MCP

MCP exposes:

- generated tools grouped under `syndocal.query.*`, `syndocal.runtime.*`,
  `syndocal.authored.*`, `syndocal.output.*`, `syndocal.file.*`, and
  `syndocal.admin.*`;
- resources for registry discovery, granted capabilities, project snapshot,
  authority/history, Timeline/Cue/Patch/media state, runtime transport, output
  ownership, recording, device health, telemetry, and terminal receipts;
- safe operator-authored prompts that propose a diff or plan before requesting
  consent for high-risk execution.

Tools expose the canonical envelope and never hide E/R/H, request identity,
owner identity, consent, or terminal state behind a smart default. Large or
high-rate data uses bounded summaries, pagination, cursors, or artifact
references; raw video frames and unbounded DMX streams are not dumped through an
ordinary MCP resource.

### 4.2 External API

The API provides:

- authenticated JSON-RPC Command/Query calls;
- generated OpenAPI-compatible REST projections where useful;
- terminal receipt query and cancellation where the operation supports it;
- snapshot plus revision/generation-stamped event subscription;
- schema/capability/version discovery;
- typed errors with stable machine codes and human detail.

Subscribers obtain a snapshot at generation N, then apply N+1 deltas. A gap,
epoch change, or unknown event forces a fresh snapshot. Events do not perform
mutations. Unknown major protocol versions, unknown fields where exactness is
required, oversized messages, and unsupported operations fail closed.

Command queue overflow is rejected before admission with a typed `overloaded`
result; accepted Commands are never silently dropped. Query concurrency is
bounded and returns `overloaded` without blocking the realtime engine. Event
fan-out is per-principal and bounded; slow subscribers receive a generation gap
marker and must resnapshot instead of backpressuring the publisher. Coalescing is
allowed only for explicitly coalescible telemetry, never terminal receipts,
authority changes, consent/revocation, or ownership events.

## 5. Authority, receipt, and audit requirements

- A client-generated request ID is mandatory for every Command.
- The backend maintains bounded single-flight and terminal receipt storage.
- An exact retry returns the stored terminal result without republishing Engine
  state, history, files, recording sessions, or output actions.
- A same-ID/different-shape request is rejected.
- Commit and terminal receipt persistence are one correctness boundary; the
  crash-after-commit/before-reply case cannot execute twice.
- Authored Commands serialize through one project writer and reject stale E/R/H.
- Runtime reports carry enough generation truth to reject response reordering.
- Redo re-enters the canonical registry contract; failed/runtime operations do
  not pollute Undo.

Receipt retention and eviction never turn a late retry into implicit execution.
After the full receipt expires, a longer-lived bounded tombstone returns
`receipt_expired`; after the documented maximum idempotency window the client must
resnapshot and submit a new request ID explicitly. All expiries, presence leases,
rate windows, credentials, and consent tokens use a monotonic clock for decisions;
wall clock is recorded only for operator/audit presentation. For owner-less file
targets, confirmation binds the canonical target-resource fingerprint,
file-operation generation, and principal incarnation instead of an output owner.

Every `R2`-`R5` and `S0` attempt records an immutable bounded audit event with principal,
adapter, request ID, operation, risk, argument fingerprint, authority/generation
before and after, consent identity, outcome code, and timestamp. Tokens, pairing
secrets, sensitive media/project contents, and paths are redacted or replaced by
stable hashes according to policy. Operators can inspect what a diagnostic export
will include.

## 6. Implementation tranches

1. **AI0 inventory and registry schema** — mechanically inventory every Tauri,
   Engine, Remote, MIDI/OSC, shortcut, audio-analysis/BPM clock, native output
   window, and UI mutation; classify gaps; introduce generated discovery with
   fail-closed unclassified entries.
2. **AI1 canonical read/query surface** — snapshots, pagination, typed errors,
   schema/version discovery, capability discovery, and generation-stamped events.
3. **AI2 authored command bridge** — unify E/R/H, shape, owner incarnation,
   terminal receipt, Undo, and exact retry across all authored families.
4. **AI3 runtime/output bridge** — runtime generations, output ownership, rate
   limits, Blackout/Arm/Takeover safety, and physical-resource idempotency.
5. **AI4 principal and consent service** — scoped grants, safe-mode default,
   single-use prepared consent, human-present state, revocation, and kill switch.
6. **AI5 localhost sidecar** — MCP stdio/streamable HTTP as supported by the
   selected MCP SDK, JSON-RPC/REST/WS adapters, bounded IPC, crash isolation, and
   no-startup-dependency behavior.
7. **AI6 desktop administration UI** — pairing, grants, safe-mode promotion,
   per-operation consent, active sessions, revoke-all, audit viewer, and status.
8. **AI7 parity and adversarial proof** — generated schema equality, GUI-to-registry
   coverage, reply-loss, stale authority, consent binding, rate/DoS, event gaps,
   sidecar crash, recording/file exactness, output ownership, and soak.
9. **AI8 native and external acceptance** — current-source native build, actual MCP
   client, JSON-RPC client, Remote migration, hardware output matrix, recovery,
   clean install, and security review.

### AI3 implementation audit checkpoint — 2026-08-19

The committed implementation at `896fb407f896edd46fe938a07a8bd5c71cfc956d`
contains substantial AI3 groundwork, but **AI3 is not complete**. The five AI3
requirements currently stand as follows:

| Requirement | Status | Implemented evidence | Open boundary |
| --- | --- | --- | --- |
| Runtime generations | Partial | Timeline Transport and Follow Abort have authority fences, generations, single-flight receipts, and bounded rate limits; query reports several additional runtime generations. | Loop/video and other legacy runtime mutations are not yet canonical or generation-fenced across Tauri, MIDI/OSC, Remote, and shortcut routes. |
| Output ownership | Partial | The machine-local output gate starts all-deny and implements role transitions, physical permits, activation/creation/teardown leases, and failure fencing. Local-GUI active Arm and exact Take Over now enter action-specific R4 operations under the lifecycle/project/output transition boundary. | The AI3 owner-incarnation lease with TTL, renew, orphaned state, stale-owner rejection, forced transfer, and restart non-reclamation is absent. The local gate is not that distributed/backend lease. |
| Rate limits | Partial | Transport, Follow Abort, S0 Blackout, and the local R4 OutputControl operations use token buckets. OutputControl has direct fake-clock proof for burst 8 / ninth rejection and per-principal/domain single flight. | MIDI/OSC/Remote and other legacy runtime routes remain outside this local vertical, and no 10,000-call realtime-budget evidence exists for the complete output path. |
| Blackout / Arm / Take Over safety | S0 plus local-GUI R4 partial | S0 engage keeps its narrow priority path. Safety-latch Release, active Arm, and exact Take Over are three one-source/one-operation R4 commands with short-lived Raw Input confirmation, exact fences/receipts, and legacy local energizing handlers fail-closed. | All/video/per-output release is deliberately unavailable pending target-aware actions. MIDI/OSC/Remote release routes, full project-swap physical retirement/re-arm, and actual hardware confirmation remain open. |
| Physical-resource idempotency | Partial | NDI, Spout, and Display creation leases recheck ownership epochs and retain teardown acknowledgement fail-closed. The local R4 operations preserve same-request terminal retry within one process, including poisoned bookkeeping-lock recovery. | Receipts and audit remain process-local/expiring, not crash-safe; restart reply-loss and the full physical project-replacement path are not proven exactly once. |

The local-GUI Take Over and R4 energizing/release boundary described below is now
race-hardened. The next safe AI3 order is: inventory and canonicalize or fail-close
the remaining MIDI/OSC/Remote and wider output routes; add the owner-incarnation
output-lease state machine; fence every project replacement through acknowledged
resource retirement and explicit Arm; then add durable receipt/audit and complete
saturation/native/hardware proof. Prepared human consent, principal grants, Raw
Input presence, revocation, and the consent service remain AI4 responsibilities.
AI3 may expose a fail-closed consent policy but must not claim those AI4 services
complete.

### AI3 local OutputControl R4 checkpoint — 2026-08-19

The checkpoint containing this section closes the **local desktop GUI vertical
only** for safety-latch Release Blackout, active Arm, and exact Standby Take Over:

- each action has a distinct Tauri source and canonical R4 operation; the public
  registry wire and discovery operation are version 3, and every R4 operation is
  constrained to exactly one local-window adapter;
- the frontend obtains a backend fence, displays a nonblocking six-digit physical
  Raw Input challenge, validates every DTO identity/expiry/fingerprint/fence, and
  retries a lost terminal reply once with the identical request object;
- legacy local GUI/backend release or energizing branches are either safer-direction
  only or fail closed. All/video/per-output release does not pretend that the
  safety-latch action covers authored blackout state;
- Arm and Release revalidate under lifecycle -> external admission -> coordinator
  -> output transition ordering and retain the output guard through commit and
  terminal-fence capture;
- Take Over revalidates exact project/output and running Standby session/generation/
  force state before any recovery-authority, input-retirement, project-publication,
  or worker-stop side effect. A stale rejection leaves the worker running; success
  commits the stop token while the validation guards are held and joins afterward;
- dedicated proof covers exact retry/shape conflicts, owner retirement, burst rate,
  single flight, lock/revalidation order, Standby status advance, consent binding,
  replay/device removal, frontend malformed DTOs, and registry/source equality.

The native completion gate for this bounded slice passed on Windows: the exact
checkout process was stopped before `pnpm --dir app tauri build --no-bundle`, the
release build succeeded, and the exact executable was relaunched with one responsive,
maximized `Syndocal` window. This is not hardware or Raw Input end-to-end acceptance.

**AI3 remains incomplete.** MIDI/OSC/Remote and wider output routes are not part of
this vertical; the owner-incarnation lease state machine, forced-transfer state,
full physical retirement/re-arm fence, crash-durable receipt/audit, saturation/soak,
and hardware matrix remain open. The local Raw Input challenge is a fail-closed R4
bridge, not completion of AI4 principals, grants, revocation, or consent service.
Windows ASIO acceptance and distribution licensing also remain separate mandatory
open product-release gates.

## 7. Required non-vacuous evidence

- A generated coverage gate fails when a non-presentational Tauri/GUI, shortcut,
  MIDI/OSC, Remote, audio-analysis/BPM, native-window, or scheduled action has no
  registry operation or a registry operation lacks risk/schema/capability data.
- Dropping an authored reply and retrying the same ID produces identical receipt,
  one revision, one publication, and one Undo entry.
- Two principals starting at revision N prove one commit and one stale rejection
  with byte-identical state after the rejected request.
- Runtime and authored cross-class payloads are rejected with no history drift.
- Consent tests cover missing, expired, wrong-principal, wrong-argument,
  wrong-generation, correct single-use, and replayed tokens.
- Kill switch tests include queued and in-flight external calls while local
  Blackout/Standby remains responsive within a measured bound.
- A 10,000-call burst is rate limited without violating the existing output tick,
  frame, audio, or UI responsiveness budgets.
- Sidecar termination during an active show leaves DMX/video/audio/recording
  ownership and output unchanged and restart does not replay old requests.
- Recording/file/project replacement reply-loss creates exactly one terminal
  artifact or transition and never overwrites an unintended target.
- Event loss is injected; the client detects a gap, resnapshots, and converges.
- MCP, JSON-RPC/REST, WebSocket, Tauri, MIDI/OSC, shortcut, and desktop routes are
  proven to call the same registered operation rather than parallel Engine paths.
- Native acceptance uses a built release executable and a real external client;
  browser mocks and schema snapshots alone do not prove completion.
- Against the built release executable, an unpaired loopback process, a forged
  PID/name, a stale sidecar session, a revoked credential, and a valid credential
  for another principal are rejected before discovery or any `R1`-`R5` actuation;
  a newly paired safe-mode principal cannot exceed its grants.
- Emergency Blackout proof distinguishes engage from release: a principal with
  only `safety.blackout.engage` can idempotently engage through the priority path,
  cannot release/toggle/Arm, and loses the capability immediately on revocation.
- Output lease proof covers renew, expiry to `held_orphaned` without output change,
  stale-owner rejection, local fenced forced transfer, retry idempotency, sidecar
  restart non-reclamation, and retained local priority Blackout.
- Queue saturation proves pre-admission `overloaded`, no accepted-command loss,
  no realtime jitter regression, and event gap/resnapshot without publisher stall.
- Release-native consent proof shows ordinary DOM/accessibility invocation,
  `SendInput`, posted window messages, Remote/API events, and stale physical input
  cannot mint presence; only the fresh Raw Input challenge can authorize `R4`/`R5`,
  and the release artifact contains no test bypass.

## 8. Release blockers and non-claims

The following are P0/P1 until closed:

- any project/output mutation bypassing the registry;
- external `R4`/`R5` execution without bound local consent;
- accepting localhost, PID, process name, or possession of a port as principal
  authentication, or exposing registry state before the authenticated handshake;
- allowing emergency Blackout engage to toggle/release output, or failing to keep
  local priority Blackout reachable during saturation/revocation;
- treating DOM/accessibility/`SendInput`/posted messages as human presence, or
  shipping a release-build bypass for the physical confirmation challenge;
- commit/receipt races that allow double execution;
- output changes caused by adapter disconnect or crash;
- unclassified commands defaulting open;
- adding a general or target-valued operation to the narrow `S0` safety class;
- an all-powerful undifferentiated external token;
- adapter-specific schemas, defaults, retries, or domain validation;
- prose-only errors or events without generation/revision;
- missing rate limits that can disturb realtime output;
- stale or crashed external owners that can renew/reclaim output without a current
  incarnation, or physical output changes caused solely by lease expiry;
- cleartext secrets or sensitive payloads in audit/diagnostics;
- runtime operations entering Undo or authored operations bypassing E/R/H.

Until AI0-AI8 are accepted, Syndocal may claim that selected backend and Remote
operations exist, but not that the whole product is AI-drivable, that MCP has full
feature parity, or that unattended high-risk automation is safe.
