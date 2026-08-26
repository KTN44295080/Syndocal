# Syndocal × rekordbox-DJ-Link-ForPCDJ acceptance

Date: 2026-08-21
Updated: 2026-08-26
Status: Required; strict DJ-Link v3 has a live authenticated initial-snapshot checkpoint on `1.2.0-alpha.15` / rb-output `1.1.5`; immutable published v1.1.3 and retired v2 sources are not current show routes; hardware acceptance remains exactly 0/12
Source authority: replacement user specifications received 2026-08-20 and 2026-08-21

## Current v3 authority — 2026-08-26

The only current wire adapter is `syndocal-envelope-v3`, using the exact frame
`{v:3,type,agentId,sessionId,sequence,eventId,payload}`. Flat, v1, and v2 frames
and adapter names are retired and rejected without a shim. HELLO advertises the
complete nine-capability set, including `DJ_LOOP_FALLBACK`; current KDMX product
metadata is `1.2.0-alpha.15` and the controlled rb-output source metadata is
`1.1.5` at `c1a1470c89088117379fcee58fb7521bffadb4c3`. Any later text that calls v2 current is retained only as dated history
and is superseded by this section and the v3 restatement near the end.

Stage 1 keeps two independent truths. A physical F14 intent arms its bounded
Rekordbox-response window before attempting local MIDI. A fresh, valid,
same-session Rekordbox measurement is authoritative and is routed as
`DJ_LOOP_STATE` with measured truth nested under `payload.loop`; the retired
flat measured-loop wire shape is rejected. Only actual no-response after the window may emit the distinct
predicted `DJ_LOOP_FALLBACK`. Invalid, stale, or contradictory same-lineage
responses suppress prediction fail-closed; a late fresh measurement overrides
and rebases any prior prediction. The exact downward profile is
`8 → 4 → 2 → 1 → 1/2 → 1/4 → 1/8 → 1/16 → 1/32 → 1/64` beats and saturates only
at `1/64`, never at 2 beats. F13 Release is separate: it routes exactly one
correlated `DJ_RELEASE` even when the local Rekordbox Stop MIDI send fails, so
Syndocal progression and Rekordbox transport results remain separately visible.

These are software contracts, not hardware evidence. The real token, wired
HELLO/ACK, controller MIDI, pedal, Rekordbox response/no-response behavior,
Release, reconnect, and restart matrix remains exactly **0/12 checked**.

Pre-v3 historical train notice (superseded by the v3 authority above): the exact KDMX
`1.2.0-alpha.12` runtime/artifact source checkpoint is
`ff61a6dec6eb5e4bc0993d9b65cd137fe1872aae`; later cleanup and documentation
checkpoints do not redefine its native artifact identity. The alpha.10 pause
was rescinded before promotion. Follow
`AGENTS.md` and `qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md` for current work.
This file remains the detailed DJ/pedal gate. The alpha.8 references in the
2026-08-24 checkpoint below are historical evidence only, not a claim that
alpha.8 is the current Syndocal artifact or authority. The live acceptance status
remains `Required / Peer and hardware pending`. Immutable v1.1.3 source/package
evidence is recorded below as historical release evidence only: its published package
does not satisfy the required `DJ_MASTER_CHANGED` retired/unreachable negative proof.
The performance is **2026-08-30**; development, acceptance, and show preparation
must be complete by the separate **2026-08-29 completion deadline**. Current
executable guidance does not wait for or use an installer: it uses only the controlled source checkout on the
target DJ PC. Peer branch `beta-v1.1.2` is clean and upstream-equal at docs tip
`e3d390d912a2c3a9be418ecbc31771d2bf515de7`; the exact runtime-source checkpoint
is `862cf8035dfb365a7d799f820936585882d0a1e7`, and the source version is `1.1.5`.
The branch name is not the product version, and the docs-only tip does not redefine
the runtime identity.
The checkout requires a show JSON outside the checkout through
`DJ_AGENT_CONFIG_PATH`. Run no-argument `start-all.bat` for the controlled real
launch; the only alternate is exact lowercase `--preflight-only`. Preflight is
software-only and intentionally launches no show-side process. The real launch
requires the current real Syndocal token; placeholder-token preflight evidence
cannot be promoted. Current show topology is FOH `192.168.50.1` and DJ PC
`192.168.50.2`. Older peer audits remain only under explicit SUPERSEDED / DO NOT
EXECUTE labels.

This document supersedes the earlier design in which a Pedal entered Syndocal first
and Syndocal sent MIDI to rekordbox. That design must not be restored.

## 1. Deployed topology and ownership

The Stream Deck Pedal, rekordbox, virtual MIDI port, global hotkeys, MIDI mappings,
filter ramp, stop, and optional post-release reset all live on the stage DJ PC. The
existing `Seraf0-org/rekordbox-DJ-Link-ForPCDJ` Node server is extended in place and
acts as the DJ Agent. No second Agent process, DLL injection, Hook UDP listener, or
parallel DJ Link server is created.

Syndocal runs at FOH. It receives authenticated semantic show events over the existing
show-network WebSocket service, owns project-specific Track-to-Timeline mappings, and
uses its existing Timeline/Loop/Video/Lighting runtime. Syndocal never sends rekordbox
MIDI, owns no Pedal/global-hotkey configuration, and does not implement the filter,
stop, Loop Off, or Filter Center reset.

```text
Stage DJ PC                                      FOH

Pedal -> rekordbox-DJ-Link-ForPCDJ               Syndocal
          |             |                            |
          |             +-- local MIDI -> rekordbox  +-- Timeline/Loop
          +-- WebSocket over wired Show LAN --------+-- Video/Lighting
```

The normal operating network is the existing managed, wired Show LAN. Host, port,
bind address, and NIC are configurable; no production address is hard-coded. Wi-Fi is
not the primary show-control path. The first release trusts the dedicated LAN and does
not claim protection against a hostile LAN without a later TLS/mTLS tranche.

## 2. Shared DJ Link v3 wire contract

DJ-Link is the authenticated WebSocket client. Syndocal extends its existing Web
Remote listener with the dedicated `/dj-link` role/path; it does not open an
unrelated second server. Generic Remote authorization and DJ Link authorization
remain separate. The only intended corrected-release adapter is
`syndocal-envelope-v3`; `generic-json`, `syndocal-envelope-v1`, and
`syndocal-envelope-v2` are retired and
must be rejected without fallback, aliases, or implicit conversion. The
controlled source route above is the only current show exception; it is not a
published-artifact or hardware-acceptance claim.

Every production frame is the exact bounded v3 envelope
`{v:3,type,agentId,sessionId,sequence,eventId,payload}`. The accepted peer identity
is `agentId:"rb-output-dj-agent"`; frames are at most 64 KiB, strings are bounded
UTF-8 without controls, and the DJ-Link token is 32..256 UTF-8 bytes. The token is
shown only through the explicit rotation flow and is never placed in a URL, query
string, ordinary status response, `.sdc`, template, backup, or Standby checkpoint.

The first Agent-to-Syndocal frame is `DJ_AGENT_HELLO`. Its exact payload contains
`authToken`, `version:3`, and this complete, duplicate-free capability set:

- `DJ_MASTER_TRACK_ACTIVE`
- `DJ_MASTER_TRACK_SYNC`
- `DJ_LOOP_STATE`
- `DJ_LOOP_FALLBACK`
- `DJ_RELEASE`
- `DJ_TIMELINE_BEAT_JUMP`
- `DJ_TIMELINE_LOOP_SET`
- `DJ_TIMELINE_STATE_REQUEST`
- `DJ_STATE_SYNC`

After HELLO, the client sends `DJ_STATE_SYNC` followed by the empty-payload
`DJ_TIMELINE_STATE_REQUEST`. Reconnect and restart use the same order. Syndocal sends
the authoritative `DJ_TIMELINE_STATE` response; no timeline action is permitted
before the snapshot is valid and ready. Legacy state-sync request aliases are
rejected; only the exact `DJ_STATE_SYNC` then `DJ_TIMELINE_STATE_REQUEST` order is
valid.

The v3 ACK shape is exact:

```json
{
  "v": 3,
  "type": "ACK",
  "eventId": "opaque-event-id",
  "sequence": 104,
  "outcome": "accepted",
  "code": null,
  "stateGeneration": 42
}
```

Allowed outcomes are `accepted`, `duplicate`, `no_mapping`, `rejected`, and `busy`;
only `accepted` and `duplicate` are successful. `busy` may retry only with the same
event identity, sequence, canonical v3 shape, and socket generation. Same event ID
plus the same canonical shape is idempotent; a changed shape, sequence rollback,
unauthenticated traffic, stale session, or session impersonation fails closed. A new
authenticated session replaces the older session by generation, and an old socket
close cannot clear the replacement. `DJ_MASTER_TRACK_SYNC` is continuous non-ACK
telemetry; physical events remain ACKed and ACK success never claims an external
physical action occurred. Heartbeat is five seconds and disconnect/timeout never
implies Release.

`DJ_MASTER_TRACK_ACTIVE` and `DJ_MASTER_TRACK_SYNC` carry the strict track payload:
`deck`, `deckId`, `masterDeckRevision`, exactly one identity form (`contentId` or
both `title` and `artist`), optional `trackBpm`, `positionAtSendSec`,
`effectiveBpm`, `positionRevision`, `sampleAgeMs`, `isPlaying:true`, `master:true`,
`startedAt`, `playSessionId`, and an optional measured loop object. The measured loop
object used by track and `DJ_LOOP_STATE` frames contains `active`, optional
`startBeat`/`endBeat`/`lengthBeats`, `revision`, `sampleAgeMs`, and the exact source
`rekordbox-hook-measured`; it is not a root-level division counter.

`DJ_STATE_SYNC.payload` contains `released`, optional `masterDeck`, and optional
`activePlaySessionId`. `DJ_TIMELINE_STATE_REQUEST` has `{}` as its payload.
`DJ_TIMELINE_BEAT_JUMP` carries `{ bars: -4|4, timelineId }`, and
`DJ_TIMELINE_LOOP_SET` carries `{ active: boolean, timelineId }`; both are
Agent-to-Syndocal and ACKed.

`DJ_TIMELINE_STATE` is Syndocal-to-Agent and authoritative. Its payload contains
`state` (`idle`, `running`, `stopped`, `ended`, or `reset`), `loopActive`,
`timelineId`, `positionBars`, `playSessionId`, `pedalOwner`, and the correlated
`releaseEventId`. State Sync updates diagnostics and measured-loop truth only; it
cannot fire a Track mapping or infer Release. Stage 2 F14 derives `active` as the
logical inverse of the latest authoritative `loopActive` for that `timelineId`;
neither the local LoopHalf action nor an ACK alone changes authority.

## 3. DJ-Link peer behavior

The peer reuses the existing hook, Hook UDP, master-change event, playback state,
Socket.IO/Web UI, packaging, and installer. Its new show-control client sends explicit
semantic events rather than the existing large browser `state` snapshot.
The peer repository is implemented and reviewed in its own Codex flow. The immutable
v1.1.3 peer source checkpoint is historical evidence only:
`5eaf1994e1bf4456857fefd36cc0ce827145b603`, with immutable release tag
`v1.1.3` at `24d38f6decbc8880149df1902ef8d2ccfe76b784`; integration acceptance pins
both identities, but the published package is blocked by its internal
`DJ_MASTER_CHANGED` mismatch. Do not install it for current acceptance; use only
the controlled target-DJ-PC source route pinned above.

The Pedal defaults may use F13/F14/F15, but remain configurable and are acquired as
native Windows global hotkeys on the DJ PC, not through browser `keydown` and not by
Syndocal.

In Stage 1, F13 runs the configured local release macro: Filter HP and the master
deck's `ChannelFader` fade, Cue/Stop, optional local reset steps, and then one
idempotent `DJ_RELEASE`. F14 keeps the local MIDI LoopHalf action and
sends `DJ_LOOP_STATE` with the absolute measured-loop object. F15 is deliberately inactive in
Stage 1 and sends neither MIDI nor a Syndocal show event. F13 macro ordering is
configuration-dependent: the documented default is `sequence:"parallel"`, while
`filter-then-fade` waits for Filter completion before starting the fade. Ramp
duration/interval and reset-after-stop policy come from the peer configuration; a
ramp or reset failure does not advance to Stop/Release. An explicitly configured
`releaseMacro.enabled:false` mode uses the direct Stop/Release action path; this is a
deliberate local mode, not an automatic legacy or wire fallback.

Only an authoritative `DJ_TIMELINE_STATE` with `state:"running"`, the current
`timelineId`/`playSessionId`, `pedalOwner:"timeline"`, and the correlated Release
event enters Stage 2.
There F13/F15 send `DJ_TIMELINE_BEAT_JUMP` with `bars:-4/+4`, while F14 sends the
absolute `DJ_TIMELINE_LOOP_SET` value derived from the latest authoritative
`loopActive`; Stage 2 never sends Rekordbox MIDI. The requested F14 value is
`active: !loopActive` for the current authoritative `timelineId`. An ACK does not
replace that authority; the next `DJ_TIMELINE_STATE` broadcast does.

During initial connection, authoritative-snapshot wait, disconnect, and immediately
after reconnect, Stage 1 F13/F14 local Rekordbox operation continues. The peer marks
only the network-side effect pending or failed and does not replay old relative
actions after reconnect; after a valid State Sync snapshot it sends `DJ_STATE_SYNC`
with current state and requests a fresh authoritative timeline state. If the State
Sync provider is invalid, neither frame is sent. If a Stage 1 `DJ_RELEASE` is pending
when the socket closes, the peer finalizes that delivery as `send-failed` for the
connection failure; the router exits `handoff-pending` to `dj-control`, reports a
failed/retryable local operation, and a later reconnect does not resend that old
physical event. A new F13 press is required. An ACK `rejected`, `timed-out`, or
`send-failed` has the same retryable Stage 1 disposition when no authoritative
running state has won the race.
If `running` was authoritative first, a late Release failure does not roll back the
completed handoff or re-enable local control.

Stage 2 cannot be entered without a valid authoritative snapshot. On disconnect it
keeps `timeline-control`, clears snapshot readiness, and does not fall back to local
MIDI; an invalid or missing snapshot does not authorize a Stage 2 action. A malformed
later state broadcast is ignored with a warning rather than becoming new authority.
After reconnect, a valid `DJ_TIMELINE_STATE(state:"running")` keeps Stage 2 active;
a valid `idle`, `stopped`, `ended`, or `reset` state exits Stage 2 to `dj-control`,
resets the peer's local loop division to zero, and restores Stage 1 (F15 inactive).

## 4. Master Track Active event

Track Loaded, Track Playing, and Master Track Active are distinct. Automatic show
mapping uses only `DJ_MASTER_TRACK_ACTIVE`, generated when the deck is current Master,
is actually playing, has a known identity, and differs from the prior active play
session. It is also generated when an already-playing deck becomes Master.

The payload uses the strict v3 fields `deck`, `deckId`, `masterDeckRevision`, exact
track identity, `trackBpm` when available, `positionAtSendSec`, `effectiveBpm`,
`positionRevision`, `sampleAgeMs`, `isPlaying:true`, `master:true`, `startedAt`,
`playSessionId`, and the optional measured loop. An explicit hook master-change wins
over explicit master state, which wins over the existing playback heuristic; the
retired `DJ_MASTER_CHANGED` name is not an accepted event in the intended corrected
v1.1.5 production v3 contract.

## 5. Project Track-to-Timeline mapping

Syndocal stores `DJ Track Trigger Mapping` in `.sdc` because show outcome is project
data. The DJ-Link peer does not store it. A mapping contains:

- stable mapping ID;
- selector: exact opaque `contentId`, or exact normalized `title` plus `artist`;
- event: `DJ_MASTER_TRACK_ACTIVE`;
- action: `StartTimeline` with an existing Timeline ID;
- retrigger policy: initially `OncePerPlaySession`.

Exact `contentId` has priority. The fallback trims and Unicode-normalizes both title
and artist and requires both to match. Title-only, fuzzy, basename, and guessed
case-fold matches are forbidden. Ambiguous selectors are rejected at save time; one
event cannot start multiple Timelines. The initial limit is 128 mappings.

The mappings are an additive, backward-compatible part of the existing
`ProjectControlMappings` CAS image and follow Save, Save As, Undo/Redo, project load,
template, backup/recovery, and Standby checkpoint paths. Connection/runtime/session
state and authentication token are process-local and are never replicated as project
data.

## 6. Syndocal event semantics

`DJ_MASTER_CHANGED` is retired and unreachable in the intended corrected v1.1.5
strict-v3 contract; it is not an advertised capability or accepted ingress event. The
immutable published v1.1.3 package is blocked because its internal encoder/router
still fails that negative proof. The corrected source lane must prove the path is
unreachable before any v1.1.5 artifact or acceptance evidence is used.
Load, preview, Cue preparation, a non-Master deck, and `DJ_STATE_SYNC` cannot start a
Timeline.

`DJ_MASTER_TRACK_ACTIVE` validates current Master/playing/identity, resolves at most
one mapping, and invokes the existing canonical runtime Timeline start path. The
dedupe key includes project epoch, mapping ID, and `playSessionId`; mapping CAS or
project replacement invalidates the prior generation.

`DJ_LOOP_STATE` is an absolute measured-loop report. Its `active`, optional
`startBeat`/`endBeat`/`lengthBeats`, `revision`, `sampleAgeMs`, and exact
`rekordbox-hook-measured` source are validated before the existing Loop runtime is
updated. Missing regions, stale samples, inconsistent bounds, and overflow fail
closed; an already-converged report is a no-op. It must never implement absolute
synchronization by repeatedly applying relative Loop Half.

`DJ_LOOP_FALLBACK` is a separately typed, absolute prediction, never a measured
loop. It is accepted only for the exact current deck/master revision/play session,
the source `pedal-no-response-predicted`, a response window from 50 through 1500 ms,
one monotonic `pedalIntentId`, the exact `baseMeasuredLoopRevision` and
`baseLoopDivision`, and one exact next value in the full `8` through `1/64`
downward profile. The receiver verifies the entire causal base before the engine
call, so an older prediction cannot overwrite a later fresh measurement. It does not
advance measured-loop revision authority; a later fresh `DJ_LOOP_STATE` overrides
it. It is rejected after Release and cannot be synthesized from invalid, stale, or
contradictory measurements.

`DJ_RELEASE` disables the current DJ loop and resumes the Timeline through the
canonical transport lane. Exact replay and repeated Release are idempotent and do not
double-advance, double-cue, or seek.

`DJ_STATE_SYNC` restores diagnostics and may converge the current measured loop only
when the synchronized state is not released. A snapshot with `released: true` cannot
re-enable or resume the loop. State Sync never replays Track
Active mappings and never executes Release transport semantics from a snapshot.

`DJ_TIMELINE_BEAT_JUMP` is accepted only for `bars:-4` or `bars:4` and the current
authoritative `timelineId`; it is available only in Stage 2. `DJ_TIMELINE_LOOP_SET`
is an absolute boolean request for that same authoritative `timelineId`, not another
relative Loop Half operation. The peer derives F14's requested boolean from
`DJ_TIMELINE_STATE.loopActive`, holds a second toggle while the first is pending, and
discards the pending request on rejection, timeout, or send failure. A successful ACK
still waits for the next authoritative timeline-state broadcast before changing the
peer's state.

## 7. Syndocal operator surface

The existing Web Remote/Setup I/O surface contains a `DJ Link agent` disclosure with:

- configured Show-LAN bind address and endpoint state;
- Connected/Disconnected, peer address, heartbeat age, and session generation;
- Master deck, playing state, current title/artist/content ID, measured Loop state;
- last event, last event age, and last terminal outcome;
- explicit token rotation/show-once workflow;
- Track mapping add/edit/remove and `Use Current Track`.

It contains no Pedal, rekordbox MIDI, Filter, Stop, reset, or global-hotkey controls.
Mappings use the existing project-authority CAS path and surface conflicts rather than
silently overwriting concurrent edits.

## 8. Automated proof

Syndocal proof must cover:

1. `/dj-link` and generic Remote role/path separation;
2. HELLO-before-use, token failure, frame bounds, rate limiting, heartbeat timeout,
   authenticated session replacement, and old-close ABA protection;
3. same-ID replay, same-ID/different-shape rejection, sequence rollback, and truthful
   terminal ACKs;
4. content-ID priority, exact title+artist fallback, title-only/non-Master/no-mapping
   rejection, and Once-per-play-session dedupe;
5. Track Load and State Sync never triggering a Timeline;
6. measured-loop revision/source freshness, non-accumulation, no-op convergence,
   exact nested active/inactive shape, retired-flat rejection, invalid
   authored-region/bounds rejection, monotonic intent/base causality, and bounded
   no-response fallback across the full `8` through `1/64` profile;
7. Release replay disabling/resuming once without seek or duplicate cue;
8. legacy project default, exact save/reload, mapping CAS conflict, template,
   backup/recovery, and Standby mapping round-trip;
9. project replacement or failed CAS producing no stale event side effect;
10. token/session/runtime state never appearing in project artifacts or ordinary
    status responses;
11. existing Remote, MIDI, OSC, WebSocket, Timeline, Video, Lighting, and warning
    ratchets remaining green.

The DJ-Link peer's immutable v1.1.3 checkpoint separately recorded Hook/Now Playing
regression safety and historical strict-v2
Master Track generation, Pedal/global-hotkey input, local MIDI mappings/ramp/reset,
local operation during disconnect, reconnect State Sync, and ACK display. The
recorded source checkpoint is
`5eaf1994e1bf4456857fefd36cc0ce827145b603` on the peer release source branch;
the immutable product release is v1.1.3, but it is historical and blocked by the
`DJ_MASTER_CHANGED` mismatch. The current strict-v3 focused test surface includes
`tests/smoke.test.js`, `tests/syndocal-envelope-v3.test.js`,
`tests/stage1-loop-fallback.test.js`, and `tests/loop-beat-projection.test.js`.
This is static/software evidence only, not a
hardware execution claim:

- Current rb-output runtime source `862cf8035dfb365a7d799f820936585882d0a1e7`
  passed full `npm test`: 389 total / 387 pass / 0 fail / 2 intentional package
  skips. Stage 1 plus strict-v3 focused tests passed 33/33, and the first-party
  warning count was 0. Independent Terra xHigh adversarial reviews caught the
  rapid-F14/inactive-loop races, late-fallback causality gap, and measured-loop
  wire-shape mismatch; the final review found P0/P1/P2 none.
  Ox-alpha was unavailable in this session; this narrow review exception is recorded
  explicitly and does not alter the hardware boundary.

- With the exact MSVC 14.44 linker pinned and first in `where.exe link.exe`, KDMX
  passed protocol DJ-Link 12/12, the dedicated rb-output sender-contract 2/2,
  I/O `remote_ws` 57/57, Syndocal DJ-Link dispatch 10/10, and extracted full-range
  mapping 3/3. All were rerun serially after the final peer wire correction; the
  first-party warning count was 0.

- `tests/smoke.test.js`:
  `timeline-control maps pedals to ACKed timeline actions without MIDI and fails
  closed on disconnect`; `Syndocal disconnect does not gate Stage 1 local MIDI
  actions`; `release handoff failures never stick in handoff-pending and running
  wins the late-failure race`; `every physical event waits for typed ACK outcomes,
  including master and timeline events`; `invalid State Sync snapshots never send
  or request timeline, then recover on reconnect`; and `Busy backoff is fenced to
  its socket and reconnect never replays old events` (6 declarations).
- `tests/syndocal-envelope-v3.test.js`: strict v3 envelope shape, typed track and
  measured-loop payloads, exact ACK outcomes, snapshot ordering, reconnect fencing,
  and Stage 1/Stage 2 fail-closed behavior.

These tests statically cover the authoritative `running` gate, F13/F15 `-4/+4`
beat-jump payloads, F14 absolute loop-set payload, no Stage 2 MIDI, invalid/missing
state handling, typed ACK rejection/timeout, and disconnect fail-closed behavior.
They do not close the physical pedal, rekordbox, wired-LAN, or two-process rows.

### SUPERSEDED / DO NOT EXECUTE — 2026-08-25 v1.1.3 peer source and distribution checkpoint (immutable historical release evidence)

This immutable v1.1.3 checkpoint records the strict-v2 peer source committed and pushed at
`5eaf1994e1bf4456857fefd36cc0ce827145b603`; the immutable annotated `v1.1.3` tag
points to `24d38f6decbc8880149df1902ef8d2ccfe76b784`. The published release contains
the installer, ZIP, and release manifest. Full `npm test` passed 328 total / 326
pass / 0 fail / 2 intentional package-smoke skips; `node --check` passed 18/18 and
first-party warnings were 0. The exact tagged artifacts and their hashes are
recorded in `C:\Users\kouty\Desktop\rb-output\SYNDOCAL_PEDAL_HANDOFF.md:75-101`.

Do not install or use this v1.1.3 package as current-final acceptance evidence: its
internal `DJ_MASTER_CHANGED` encoder/router mismatch blocks the required negative
proof. The replacement source version 1.1.5 is pushed on `beta-v1.1.2`; runtime
source is `862cf8035dfb365a7d799f820936585882d0a1e7` and the clean upstream-equal
docs tip is `e3d390d912a2c3a9be418ecbc31771d2bf515de7`. No v1.1.5 tag, package, or
published release is the current show route.

This checkpoint is software/package evidence only. It does not close the physical
pedal, Rekordbox, two-process, wired-LAN, reconnect, restart, or shared-network rows;
the HW-4 matrix remains exactly **0/12 checked (0%)**.

### 2026-08-26 alpha.15 FOH/DJ transport preflight

- The exact standard alpha.15 Syndocal process was running from this checkout as
  PID `34792`; its single `Syndocal` window was responsive and maximized before
  native UI inspection. No Web Remote/DJ listener was started.
- The intended Show-LAN adapter was up as `192.168.50.1/24` on `イーサネット 4`
  (`ifIndex 3`, `1 Gbps`). The target DJ PC `192.168.50.2` did not answer the
  bounded one-packet reachability probe, and no local listener was present on TCP
  `8787` or `9100`. UDP `22346` was not inspected in this resumed preflight.
- `C:\Users\kouty\Desktop\rb-output` remained clean and upstream-equal at
  `e3d390d912a2c3a9be418ecbc31771d2bf515de7`. The target-DJ-PC-only external
  configuration `C:\SyndocalShow\dj-agent-v1.1.5.json` was absent on this FOH
  machine and `DJ_AGENT_CONFIG_PATH` was unset. The real process-local token was
  not exposed, copied, logged, or rotated during this preflight.
- `pnpm --dir app run check:dj-link` passed with no first-party warning. Static
  route tracing confirmed that the controlled live sequence is `Enable DJ Link`
  -> refresh/select `192.168.50.1` -> rotate/copy the show-once token -> start the
  shared Web Remote/DJ listener. Token rotation requires an explicit operator
  confirmation because it invalidates the current credential. The copied token
  belongs only in the checkout-external JSON on the actual `.50.2` DJ PC.
- Moving the physical controller before this peer session is established remains
  safe for software work, but it accepts no HW-4 row. Rows HW-4.5 through HW-4.9
  still require the controller/pedal and configured `CustomMIDI1` route; all
  twelve rows remain unchecked.

### 2026-08-26 alpha.15 live listener activation

- After the transport preflight above, the operator-approved live start was
  verified in the exact alpha.15 Syndocal process from this checkout: PID `34792`
  owns the TCP listener bound to `192.168.50.1:9100`.
- The maximized Setup I/O surface now shows `Remote Stop` and `DJ Link Available`.
  No PIN or token value is recorded here.
- The operator-approved token rotation completed, but copying the show-once
  value was not verified. A fresh rotation and direct copy into the real
  `.50.2` checkout-external configuration are still required before HELLO/ACK.
- This proves only the FOH-side listener start. Physical DJ acceptance remains
  exactly **0/12**; the `.50.2` peer and the real `rb-output` connection remain
  unverified.
- This live observation is limited to the lifetime of PID `34792`. Listener and
  token restoration after stop/restart remain unproven, so HW-4.11 is still
  unchecked within the `0/12` matrix.

### 2026-08-26 strict-v3 initial snapshot and Timeline mapping-selection checkpoint

- The target DJ PC ran the clean upstream-equal `beta-v1.1.2` source checkout at
  `c1a1470c89088117379fcee58fb7521bffadb4c3`. Its status reported
  `Enabled=true`, `SyndocalState=connected`, `StateSync=sent`, no last error,
  `TimelineState=idle`, `TimelineSnapshotReady=true`, and no Timeline warning.
  The operator-supplied DJ-PC screenshot records `SYNDOCAL CONNECTED`,
  `MIDI CONNECTED`, `PEDAL MODE DJ-CONTROL`, `TIMELINE STATE IDLE`,
  `TIMELINE LOOP OFF`, and `Ready` in one frame. The preserved evidence is
  `qa/artifacts/dj-pc-initial-timeline-snapshot-20260826.png`, 89,652 bytes,
  SHA-256 `A5E610148CC4F16C1733DA41C9B9034DBB424C552D39F8DCD2C7CEE1F6F2EAEE`.
- This proves a real authenticated v3 connection and authoritative initial
  Timeline snapshot. It does not prove HW-4.1's session-replacement and old-close
  requirements, so no HW-4 checkbox changes and the matrix remains exactly 0/12.
- During mapping setup, the authored Timeline selection visibly returned to the
  placeholder after a Setup snapshot poll. The old route mapped
  `snapshot().timeline_bank` inline on every poll, recreating Solid `<For>` option
  identities while leaving the draft signal potentially stale. The new route in
  `7063db5` projects options through the extracted `djTimelineOptions.ts` helper
  and retains the exact array and option identities while the authored IDs and
  labels are unchanged. The reason is to keep the visible select and the mapping
  draft aligned across equivalent snapshot replacement; invalid non-positive IDs
  remain excluded and an actual authored option change remains visible.
- `pnpm --dir app run check:dj-link`, localization 3553/3553, TypeScript/Vite
  build, and `pnpm --dir app tauri build --no-bundle` passed. The native build
  used the exact Community MSVC 14.44 linker pin with that linker first in
  `where.exe link.exe`; first-party warning count was 0. Independent Ox-alpha
  review returned APPROVE with no P0/P1 findings. The rebuilt exact-checkout
  executable launched as one responsive `Syndocal` window, PID `46120`; after
  selecting Timeline 1 and waiting through polling, the operator confirmed that
  the selection remained.
- The rebuild necessarily disconnected the pre-build WebSocket. Listener/token
  re-arming, current-track capture, mapping persistence, Track Active, pedal,
  Release, reconnect fencing, and shared-network acceptance remain open. Do not
  operate the pedal until the new process is connected and the exact temporary
  track mapping is visible.

## 9. Native and hardware acceptance

End-to-end acceptance records both repository commits/artifacts, Windows and app
versions, DJ/FOH NICs and switch path, rekordbox and Stream Deck versions, Pedal model
and firmware, virtual MIDI device/mapping, operator/date, packet/log timestamps, and
video evidence. It demonstrates Track pre-load without trigger, actual Master playback
trigger, Master switch, repeated absolute measured-loop reports, filter isolation, Release,
disconnect/local operation/reconnect sync, same-session dedupe, app restart, and next
show reuse while Art-Net/sACN traffic shares the wired network.

The HW-4 demonstration matrix is explicit below. All twelve rows are still unchecked:
the current software tests do not substitute for a real pedal, rekordbox, two-process,
wired-LAN, or shared-network run. Therefore the DJ/Pedal matrix is **0/12 checked
(0%)**, with **12/12 Required / Peer and hardware pending**. This is a local HW-4
matrix count and does not add or remove any item from the whole-product accepted
denominator, which remains **19/71 (26.8%)**.

| Check | Required demonstration | Status |
| --- | --- | --- |
| [ ] HW-4.1 | Wired `/dj-link` HELLO/authentication, session replacement, and old-close protection | Required / Peer and hardware pending |
| [ ] HW-4.2 | Track pre-load without trigger and non-Master rejection | Required / Peer and hardware pending |
| [ ] HW-4.3 | Actual Master playback emits one mapped Track Active event | Required / Peer and hardware pending |
| [ ] HW-4.4 | Master switch while already playing | Required / Peer and hardware pending |
| [ ] HW-4.5 | Stage 1 F14 local LoopHalf plus repeated absolute measured-loop `DJ_LOOP_STATE` reports | Required / Peer and hardware pending |
| [ ] HW-4.6 | Stage 1 F13 Filter isolation and configured local release-macro behavior | Required / Peer and hardware pending |
| [ ] HW-4.7 | Stage 1 F13 Release, ACK/rejection/timeout, and retry disposition | Required / Peer and hardware pending |
| [ ] HW-4.8 | Stage 2 authoritative `running`; F13/F15 `-4/+4`, F14 absolute loop set, and no MIDI | Required / Peer and hardware pending |
| [ ] HW-4.9 | Disconnect/local Stage 1 operation, reconnect State Sync, and Stage 2 fail-closed behavior | Required / Peer and hardware pending |
| [ ] HW-4.10 | Same-session event dedupe and replay safety | Required / Peer and hardware pending |
| [ ] HW-4.11 | App restart and next-show reuse | Required / Peer and hardware pending |
| [ ] HW-4.12 | Art-Net/sACN traffic sharing the wired network during the DJ run | Required / Peer and hardware pending |

The separately developed DJ-Link peer has no current-final accepted release. The
immutable v1.1.3 package is blocked by its `DJ_MASTER_CHANGED` mismatch. Source
version 1.1.5 is pushed on branch `beta-v1.1.2`; the controlled runtime-source
checkpoint is `862cf8035dfb365a7d799f820936585882d0a1e7`, while the clean upstream-equal
docs tip is `e3d390d912a2c3a9be418ecbc31771d2bf515de7`. For the 2026-08-30 performance, with preparation
complete by 2026-08-29, the only permitted path is that
target-DJ-PC source checkout with the checkout-external configuration and real
current token described above, not an installer. Until identity binding and the
wired-LAN hardware matrix pass, this feature remains `Required / Peer and
hardware pending`; Syndocal-side automated completion is not an end-to-end
completion claim.

## SUPERSEDED / DO NOT EXECUTE — 2026-08-24 software/package checkpoint (historical alpha.8-era evidence)

This section is retained as historical evidence only. Do not use its artifacts,
ports, versions, or process observations for current acceptance.

The separately developed DJ-Link peer package `1.1.0` was committed and pushed
on `Beta` at `6c4f4328a6866d9d48022bd8ee20a7887c9de851`. Its package/runtime checks passed:
54 tests, 16 Node syntax checks, and zero warnings. The packaged
`C:\Users\kouty\Desktop\rb-output\dist\server.exe` is SHA-256
`339ECF6E82EB463F55977F63A137CB0CB52886CD7E2874E87F5AD4724234377B`. This
closes only the peer software/package checkpoint; it does not promote any
hardware or wired-LAN row.

At this checkpoint on this Windows machine, rekordbox was not running and the
Stream Deck Pedal was not detected. The old peer process remained on
UDP 22346/HTTP 8787; no production cutover was claimed, and the known Daslight
and Ableton processes were left untouched. Syndocal's then-current alpha.8
native build/reload and completed five-display pane-route evidence (historical
checkpoint) are recorded separately in
`qa/artifacts/native-physical-acceptance/2026-08-24-alpha8-current-source-final/native-display-route-current-source.json`
(SHA-256 `41F1D6E2528E7439657F8879F753255221E25F2DF0474139B56C1570E2C32C41`);
neither substitutes for the DJ/rekordbox/State Sync/Pedal matrix.

The alpha.8 staged-source authority at that historical checkpoint was
`qa/artifacts/source-freeze/2026-08-24-alpha8-current-source-freeze.sha256`, with
103 payload records and its own manifest envelope excluded to avoid recursive
self-hashing. The alpha.7 pre-alpha.8 manifest is historical only. The display
route accepts placement/maximize/containment only: all five pane captures still
show the owner-registration status at six seconds, so transactional pane
operation, warning-clean pane startup, and completed owner registration remained
unaccepted alpha.9 work at that checkpoint.

Therefore the authoritative status remains `Required / Peer and hardware
pending`. Still-open evidence includes wired-LAN HELLO/auth/session replacement,
real Master playback and switch, absolute Loop divisions, Release, disconnect
and reconnect State Sync, Pedal/global-hotkey input, app restart/next-show reuse,
and concurrent Art-Net/sACN traffic. No DJ/Pedal completion claim is made and the
KDMX accepted denominator remains 19/71 (26.8%).

## SUPERSEDED / DO NOT EXECUTE — 2026-08-25 peer v1.1.1 historical unbound software/package smoke

This section is retained as historical evidence only. Do not select v1/generic-json,
use its artifacts, or treat its smoke endpoints as current acceptance guidance.

The rekordbox-DJ-Link peer source is at `cdd90e1e` on `main`, with the matching
`v1.1.1` tag and `origin/main` at `0` ahead / `0` behind. `npm test` passed
`69/69`. The official `npm run build:hook` path first exposed the bundled
TDM-GCC 10.3 `GetTickCount64` declaration failure; rerunning the same hook build
through the Visual Studio 2022 x64 environment succeeded without a source
change, and `npm run build:dist` completed with exit `0`. The rebuilt
packaged executable is
`C:\Users\kouty\Desktop\rb-output\dist\server.exe`, size `65,975,425` bytes,
SHA-256
`C966CE8AFAC4A54A9A8C818D75A829A2063EA92D3F434C8C31F1C0A8404BE36B`. The
fallback package (Inno Setup was unavailable on this machine) is
`C:\Users\kouty\Desktop\rb-output\dist\rb-output-20260825.zip`, size
`62,462,263` bytes, SHA-256
`6C89C727C51E7E45ACE4A850C0934906E5119D86641380BAA2E1BFF7CC855000`.

An isolated packaged smoke run on port `8788` with
`SYNDOCAL_WS_ADAPTER=syndocal-envelope-v1` and `DJ_AGENT_ENABLED=true` returned
HTTP 200 from `/api/health`, the packaged root `/`, and
`/api/dj-agent/status`; the health response reported version `1.1.1` and
commit `cdd90e1e`, with no token exposure. The
existing source service on port `8787` (PID `97208`) was not stopped. First-party
compile warnings were `0`; PyInstaller platform/optional diagnostics were `256`,
and the Node `ExperimentalWarning` count was `1`.

Production must explicitly select `syndocal-envelope-v1`; the peer's default
`generic-json` adapter is not the accepted production wire policy until that
policy is deliberately changed and re-accepted. This checkpoint is software and
packaging evidence only: no real hardware acceptance or Syndocal two-process
acceptance is claimed, and all physical/wired-LAN gates above remain unchecked.
The reported version/commit fields came from a mutable adjacent/runtime identity
path and were not cryptographically or structurally bound into that executable;
the later audit below is authoritative for provenance acceptance.

## SUPERSEDED / DO NOT EXECUTE — 2026-08-25 updated DJ-Link peer audit (v1.1.1 historical evidence)

This section is retained as historical evidence only. Its v1.1.1 identity and
generic-json/v1 interop statements are superseded by the intended corrected v1.1.4
strict-v2 contract above; the immutable v1.1.3 package is blocked and is not current
acceptance evidence.

Read-only re-audit of the separately developed DJ-Link peer at
`C:\Users\kouty\Desktop\rb-output`: the checkout is clean on `main`, matching
`origin/main` and tag `v1.1.1`, at commit `cdd90e1`. The diff from the
previously audited `616c89792016a1c17c94ebd20e8cf8de3aea5ece` contains only
package/package-lock/installer/README/Hook DLL source display-version changes; there
are no server/dj-agent/`syndocalClient.js`/config/wire changes.

Static interop is unchanged: the current flat `/dj-link` generic-json contract
and the optional `syndocal-envelope-v1` contract remain statically compatible
with Syndocal's dedicated `/dj-link` role/path and envelope semantics.

Peer-side gates rerun green: `npm test` passes 69/69, the envelope-focused
tests pass 9/9, and `git diff --check` passes; the only observed warning is
the Node MockTimers ExperimentalWarning.

A `dist/server.exe` exists with SHA-256
`C966CE8AFAC4A54A9A8C818D75A829A2063EA92D3F434C8C31F1C0A8404BE36B`, but the
peer build does not embed Git SHA/fingerprint, so this executable must NOT be
claimed to belong to `cdd90e1`.

This audit requires no Syndocal implementation change. It closes only the peer
source/package re-review row. An identity-bound peer artifact (build-time
HEAD/fingerprint binding) and the physical wired-LAN matrix remain open, so the
authoritative status stays `Required / Peer and hardware pending`; no
end-to-end DJ/Pedal completion claim is made by this record.

## SUPERSEDED / DO NOT EXECUTE — 2026-08-25 live-LAN preflight (historical observation; zero accepted HW-4 rows)

This section records an older development peer and a read-only preflight only. Do not
use its process, endpoint, version, or 404 observations as current acceptance state.

The KDMX checkout was clean on `codex/syndocal-v1.2` at
`1402a93069d8d630df66b1f682d0813b2d595faa`, equal to
`origin/codex/syndocal-v1.2`. The running exact-checkout alpha.11 executable was
PID `114780`, size `57,888,768` bytes, and SHA-256
`522074E96A235310C9D39D3200E2A9D9B5C68429F0160E92318B568A0FE5AEE2`, matching
the accepted main-checkout artifact above. A read-only socket inventory found no
TCP `9100` listener. This is not evidence that the `/dj-link` listener is absent
from the build: the executable contains that route and its strict HELLO contract.
The production state starts with Web Remote stopped, while the renderer defaults
DJ Link to disabled with no selected bind IP; only the explicit `Start Remote`
operation creates the shared Web Remote/DJ Link listener.

The current FOH Show-LAN target is Ethernet 3, interface index `28`, IPv4
`192.168.1.34/24`. With the default port, the DJ PC must connect to
`ws://192.168.1.34:9100/dj-link`. The Ethernet 3 Windows network category was
`Public`, and the read-only inspection found neither a Syndocal/KDMX firewall
rule nor an exact TCP `9100` port rule. A successful local bind therefore must
not be treated as proof that the DJ PC can reach the endpoint. Before the live
HELLO gate, use a dedicated trusted network profile and a narrowly scoped inbound
allowance for the exact Syndocal executable, TCP `9100`, and the DJ PC address;
record any profile or firewall mutation as acceptance evidence.

The connected DJ PC was running a development, not yet release-verified,
`rb-output` 1.1.2 state. Its Agent was enabled; the `CustomMIDI1` port 1 route
reported ready; the Pedal listener was armed for F13/F14/F15; and the Hook path
reported Deck 2, BPM, playback, Loop, and mixer observations. F13 and F14
operations were recorded and their local MIDI sends occurred. Their Syndocal
send attempts failed because the Syndocal endpoint was disabled and not
listening, so none of these observations proves a `/dj-link` HELLO, authenticated
session, ACK, Timeline action, or reconnect path. The new setup API returned HTTP
404 from the running peer, showing that the observed process was an older build
than the current setup surface. It must be replaced by an identity-bound corrected
v1.1.4 artifact once that release is tagged and published; the 404 must not be
described as a passing setup check. The immutable published `v1.1.3` release is
historical but blocked by its `DJ_MASTER_CHANGED` mismatch; this observation checks
no hardware row.

## Current KDMX runtime/operator preflight gaps

Two product gaps remain explicit. **P1:** Web Remote/DJ Link enabled state, bind
selection, and listener start are not restored on application launch, while the
machine-local token is regenerated for each Syndocal process. A previously
configured peer therefore cannot satisfy HW-4.11 restart/next-show reuse without
manual token rotation and reconfiguration. Completion requires secure
machine-local secret storage outside `.sdc`, renderer storage, logs, URLs, and
ordinary status; NIC identity plus address revalidation; stale-NIC fail-closed
behavior; an explicit armed/autostart policy; and restart/reconnect proof.
**P2:** the current address-only interface picker excludes loopback and link-local
addresses but can still offer virtual adapters such as the observed WSL address
`172.30.208.1`. It should expose adapter identity and reject or explicitly warn on
virtual/tunnel candidates so the operator cannot silently bind the wrong network.

## Current show wire authority — exclusive `syndocal-envelope-v3` (restated 2026-08-26)

This is the current, executable show contract; it is deliberately restated
outside every SUPERSEDED / DO NOT EXECUTE label. The shipped/current/production
wire between the DJ-PC peer and Syndocal is exclusively `syndocal-envelope-v3`.
`generic-json`, `syndocal-envelope-v1`, and `syndocal-envelope-v2` are retired and must be rejected
explicitly without fallback, aliases, diagnostic selections, or implicit
conversion. The exact v3 frame is
`{v:3,type,agentId,sessionId,sequence,eventId,payload}`. An authenticated
session becomes ready only after `DJ_AGENT_HELLO`, an authoritative
`DJ_STATE_SYNC`, `DJ_TIMELINE_STATE_REQUEST`, and the corresponding canonical
timeline-state response. Missing, unknown, stale, reordered, or legacy-shaped
frames fail closed without fallback or implicit conversion.

This restatement moves the same authority out of the historical 2026-08-25
correction record below, which stays under its SUPERSEDED label as history
only. No HW-4 row is changed by this restatement: the matrix remains **0/12
checked (0%)**, every row remains `Required / Peer and hardware pending`, and
the whole-product accepted denominator remains **19/71 (26.8%)**.

## Current v1.1.5 controlled-source strict-v3 hardware acceptance preflight

The next acceptance action uses only the clean, upstream-equal target-DJ-PC source
checkout on `beta-v1.1.2` at docs tip
`e3d390d912a2c3a9be418ecbc31771d2bf515de7`; its exact strict-v3 runtime source
is `862cf8035dfb365a7d799f820936585882d0a1e7`, source version `1.1.5`. Do not install the blocked
immutable v1.1.3 package and do not substitute a shortcut or installer. In the
same PowerShell, set `DJ_AGENT_CONFIG_PATH` to the checkout-external show JSON.
Use exact no-argument `start-all.bat` for the real controlled launch; exact
lowercase `--preflight-only` is the only alternate. A passing preflight proves
only source/configuration software checks and deliberately takes no show-side
process, LAN, Rekordbox, MIDI, pedal, or Syndocal ACK action.
No controlled-source preflight or real launch has been promoted by this checkpoint;
those observations remain explicitly unverified until run on the target DJ PC.

For the real run, replace every placeholder with the current real Syndocal token,
use FOH `192.168.50.1` and the target DJ-PC NIC `192.168.50.2`, and select only
`syndocal-envelope-v3`. Then prove the strict v3 wired HELLO / STATE_SYNC /
TIMELINE_STATE_REQUEST / ACK path before advancing through HW-4.1 to HW-4.12.
No generic-json or v1 fallback is permitted. This preflight changes no checkbox:
the DJ/Pedal matrix remains exactly **0/12 checked (0%)**, all twelve rows remain
`Required / Peer and hardware pending`, and the whole-product accepted
denominator remains exactly **19/71 (26.8%)**.

## SUPERSEDED / DO NOT EXECUTE — 2026-08-25 strict v2 adapter authority correction (historical)

Historical record only. The current, executable restatement of this authority
is "Current show wire authority — exclusive `syndocal-envelope-v3`" above.

The shipped/current/production wire is exclusively `syndocal-envelope-v2`.
`generic-json` and `syndocal-envelope-v1` are retired and must be rejected
explicitly; they are not compatibility or diagnostic selections. The exact v2
frame is `{v:2,type,agentId,sessionId,sequence,eventId,payload}`. An authenticated
session becomes ready only after `DJ_AGENT_HELLO`, an authoritative
`DJ_STATE_SYNC`, `DJ_TIMELINE_STATE_REQUEST`, and the corresponding canonical
timeline-state response. Missing, unknown, stale, reordered, or legacy-shaped
frames fail closed without fallback or implicit conversion.

This historical correction records that the former docs-only paragraph that named
flat `generic-json` as production was factually wrong and is removed as a clean break.
Any still-visible earlier v1 or
generic-json references in dated observations describe only what an obsolete
peer exposed at that time; they are not executable guidance and cannot be used
for acceptance. This correction checks no HW-4 row: the DJ/Pedal matrix remains
exactly **0/12 checked (0%)**, all rows remain `Required / Peer and hardware
pending`, and no wired-LAN, MIDI, pedal, Rekordbox, or Syndocal ACK acceptance is
claimed.
