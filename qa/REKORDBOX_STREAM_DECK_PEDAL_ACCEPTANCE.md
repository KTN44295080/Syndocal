# Syndocal × rekordbox-DJ-Link-ForPCDJ acceptance

Date: 2026-08-21
Updated: 2026-08-27
Status: Required; strict DJ-Link v3 software gates are current on KDMX `1.2.0-alpha.18` at source commit `db4eefc348b01ee05dd2dc87945afa85de8803e` (native build/launch/layout verified; direct refresh click unconfirmed) and controlled rb-output `1.1.8` commit/full-suite evidence is accepted at the pinned peer identity; target-DJ-PC deployment and the physical matrix remain unaccepted, with hardware acceptance exactly 0/12
Source authority: replacement user specifications received 2026-08-20 and 2026-08-21

## Current v3 authority — 2026-08-26

The only current wire adapter is `syndocal-envelope-v3`, using the exact frame
`{v:3,type,agentId,sessionId,sequence,eventId,payload}`. Flat, v1, and v2 frames
and adapter names are retired and rejected without a shim. The new Agent HELLO
advertises the complete nine-capability set headed by `DJ_TRACK_ACTIVE` and
`DJ_TRACK_SYNC` and including `DJ_LOOP_FALLBACK`; the current KDMX source
checkpoint is `1.2.0-alpha.18` at branch `codex/syndocal-v1.2`, exact
`HEAD`/upstream `db4eefc348b01ee05dd2dc87945afa85de8803e`, with native
build/launch/layout verified and the direct refresh click unconfirmed. The
alpha.17 source checkpoint is
`fb5d18fdf898a1435bed173ddd17934a04a97897`; its native artifact/process
evidence recorded below remains historical and is not re-bound to alpha.18.
The controlled rb-output `1.1.8` source is pinned to
  peer commit `0f3e8c6851857c8542c132a89a7d44289002b1f5`; its stable suite passed
  `415` total / `413` pass / `0` fail / `2` intentional skips. Peer software gates
  passed, but no alpha.18 native build, target-DJ-PC deployment, or physical row
  is accepted.
  The live target-DJ-PC evidence remains on source version `1.1.5`.
Any later text that calls v2 current is retained only as dated history
and is superseded by this section and the v3 restatement near the end.

Stage 1 keeps two independent truths after a mapped `DJ_TRACK_ACTIVE` ACK admits
one exact deck/deckId/playSessionId owner. A physical F14 intent arms its bounded
Rekordbox-response window before attempting local MIDI. A fresh, valid,
same-session Rekordbox measurement is authoritative and is routed as
`DJ_LOOP_STATE` with measured truth nested under `payload.loop`; the retired
flat measured-loop wire shape is rejected. Only actual no-response after the window may emit the distinct
predicted `DJ_LOOP_FALLBACK`. Invalid, stale, or contradictory same-lineage
responses suppress prediction fail-closed; a late fresh measurement overrides
and rebases any prior prediction. The exact downward profile is
`8 → 4 → 2 → 1 → 1/2 → 1/4 → 1/8 → 1/16 → 1/32 → 1/64` beats and saturates only
at `1/64`, never at 2 beats. On an accepted Stage 1 F13 edge, HPF CC16 begins
and exactly one correlated `DJ_RELEASE` is routed synchronously before local
MIDI completion. Syndocal only turns the admitted DJ loop OFF and relinquishes
the DJ clock when the Timeline is already playing; it does not seek, jump,
start, or otherwise change position, playing state, child transport, or Follow.
The local DJ leg then runs HPF, ChannelFader CC17 fade, Cue/Stop, and HPF/fader
reset independently. MIDI failure never gates Release delivery.

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
executable guidance does not wait for or use an installer: it uses only the
controlled source checkout on the target DJ PC. Peer branch `beta-v1.1.2` must
be clean and upstream-equal at source version `1.1.8`, with both `HEAD` and
`@{upstream}` exactly `0f3e8c6851857c8542c132a89a7d44289002b1f5`. The stable peer
suite passed `415` total / `413` pass / `0` fail / `2` intentional skips. The
branch name is not the product version, and a docs-only tip does not redefine
runtime identity.
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

- `DJ_TRACK_ACTIVE`
- `DJ_TRACK_SYNC`
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
close cannot clear the replacement. `DJ_TRACK_SYNC` is continuous non-ACK
telemetry; physical events remain ACKed and ACK success never claims an external
physical action occurred. Heartbeat is five seconds and disconnect/timeout never
implies Release.

`DJ_TRACK_ACTIVE` and `DJ_TRACK_SYNC` carry the strict any-deck payload: `deck`,
`deckId`, exactly one identity form (`contentId` alone or both `title` and `artist`
without `contentId`), optional `trackBpm`, `positionAtSendSec`, `effectiveBpm`,
`positionRevision`, `sampleAgeMs`, `isPlaying:true`, `startedAt`, `playSessionId`,
and an optional measured loop object. They carry neither `master` nor
`masterDeckRevision`; Master state is not trigger authority. The measured loop
object used by track and `DJ_LOOP_STATE` frames contains `active`, optional
`startBeat`/`endBeat`/`lengthBeats`, `revision`, `sampleAgeMs`, and the exact source
`rekordbox-hook-measured`; it is not a root-level division counter.
Optional `trackBpm` and track-level `loop` values may be omitted or JSON `null`;
no other substitute shape is accepted.

The nine capabilities above are the sole advertised and emitted show contract.
There is no Master-only capability family, event enum, payload DTO, runtime
field, status projection, UI diagnostic, or compatibility shim. A former
Master-only HELLO, `DJ_MASTER_TRACK_ACTIVE`/`DJ_MASTER_TRACK_SYNC`,
`masterDeckRevision`, `masterDeck`, and a mixed capability set are all rejected
at strict ingress. The positive companion proof is the generic exact any-deck
HELLO and mapped `DJ_TRACK_ACTIVE`; this software proof does not close HW-4.

For the new generic capability set, `DJ_STATE_SYNC.payload` contains `released`
and optional correlated `ownerDeck`, `ownerDeckId`, and `activePlaySessionId`.
The three owner fields are all present together or all omitted; partial and
explicit-null owner triples fail closed. `masterDeck` is not a supported alias
or persisted compatibility field. `DJ_TIMELINE_STATE_REQUEST` has `{}` as its payload.
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

The peer reuses the existing hook, Hook UDP, per-deck playback state,
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

In Stage 1, the accepted F13 edge begins the local HPF ramp and synchronously
routes exactly one correlated, idempotent `DJ_RELEASE` before local MIDI can
complete. The independent local release macro then runs against the admitted
owner deck: HPF, that deck's `ChannelFader` fade, Cue/Stop, and HPF/fader reset.
The current v1.1.8 profile is strict: `releaseMacro.enabled=true`,
`sequence:"filter-then-fade-then-stop"`, and `releaseFade.enabled=true`. HPF CC16 ramps
`64 -> 127`; after HPF completes, the independent ChannelFader CC17 leg ramps
`127 -> 0`, each over `1000 ms` with `50 ms` updates, followed by Cue/Stop Note37
and resets to HPF `64`/fader `127`. A local MIDI failure remains visible but
never suppresses the already-routed Release. F14 keeps the local MIDI LoopHalf action and
sends `DJ_LOOP_STATE` with the owner deck/deckId/playSessionId and absolute
measured-loop object. No Track Active ACK other than `accepted` or `duplicate` may
establish that owner, and an unrecognized concurrent deck cannot steal it. F15 is deliberately inactive in
Stage 1 and sends neither MIDI nor a Syndocal show event. Other macro sequences
or direct Stop/Release are retired v1.1.7 or older profiles and must be rejected
for current acceptance.

Only an authoritative `DJ_TIMELINE_STATE` with `state:"running"`, the current
`timelineId`/`playSessionId`, `pedalOwner:"timeline"`, and the correlated Release
event enters Stage 2.
There F13/F15 send `DJ_TIMELINE_BEAT_JUMP` with `bars:-4/+4`, while F14 sends the
absolute `DJ_TIMELINE_LOOP_SET` value derived from the latest authoritative
`loopActive`; Stage 2 never sends Rekordbox MIDI. This existing
`timeline-control` beat-jump boundary is unchanged by v1.1.8 and remains to be
confirmed directly; it is not the Stage 1 Release path. The requested F14 value is
`active: !loopActive` for the current authoritative `timelineId`. An ACK does not
replace that authority; the next `DJ_TIMELINE_STATE` broadcast does.

During initial connection, authoritative-snapshot wait, disconnect, and immediately
after reconnect, Stage 1 F13/F14 local Rekordbox operation continues. The peer marks
only the network-side effect pending or failed and does not replay old relative
actions after reconnect; after a valid State Sync snapshot it sends `DJ_STATE_SYNC`
with current state and requests a fresh authoritative timeline state. After that
fresh snapshot, it re-announces every still-fresh actually-playing exact candidate
once for the new connection generation. A surviving receiver dedupes the same
deck/session without restarting; a restarted receiver can re-admit it from the
persisted mapping. It never re-announces stale, non-playing, ambiguous, or released
sessions. If the State Sync provider is invalid, neither frame is sent. If a Stage 1 `DJ_RELEASE` is pending
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

## 4. Any-Deck Track Active event

Track Loaded, Track Playing, and Track Active are distinct. Automatic show mapping
uses only `DJ_TRACK_ACTIVE`, generated once for each fresh deck/play session when the
deck is actually playing and has one exact known identity. Current Rekordbox Master,
Master transfer, preview, load, and Cue preparation do not gate or synthesize it.

The payload uses the strict v3 any-deck fields described above. A playing deck that
has only title, has ambiguous identity, carries contentId together with title/artist,
or lacks a fresh position sample fails closed. `DJ_TRACK_SYNC` then advances only
the exact admitted deck/deckId/playSessionId with a monotonic position revision.
The retired `DJ_MASTER_CHANGED` name is not accepted in the corrected production v3
contract.

## 5. Project Track-to-Timeline mapping

Syndocal stores `DJ Track Trigger Mapping` in `.sdc` because show outcome is project
data. The DJ-Link peer does not store it. A mapping contains:

- stable mapping ID;
- selector: exact opaque `contentId`, or exact normalized `title` plus `artist`;
- event: `DJ_TRACK_ACTIVE`;
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
Load, preview, Cue preparation, a non-playing deck, and `DJ_STATE_SYNC` cannot start
a Timeline. A non-Master deck is deliberately eligible when it is actually playing
an exact mapped track.

`DJ_TRACK_ACTIVE` validates playing/identity/freshness, resolves at most one mapping,
and invokes the existing canonical runtime Timeline start path. The dedupe key
includes project epoch, mapping ID, deck/deckId, and `playSessionId`; mapping CAS or
project replacement invalidates the prior generation. A mapping ACK of `accepted`
or `duplicate` admits that exact deck/session as pedal and synchronization owner.
`no_mapping`, `rejected`, `busy`, timeout, and send failure do not establish or
replace ownership. Until Release or an authoritative terminal lifecycle clears the
owner, another concurrent candidate cannot steal it.

`DJ_LOOP_STATE` is an absolute measured-loop report. Its `active`, optional
`startBeat`/`endBeat`/`lengthBeats`, `revision`, `sampleAgeMs`, and exact
`rekordbox-hook-measured` source are validated before the existing Loop runtime is
updated. Missing regions, stale samples, inconsistent bounds, and overflow fail
closed; an already-converged report is a no-op. It must never implement absolute
synchronization by repeatedly applying relative Loop Half.

`DJ_LOOP_FALLBACK` is a separately typed, absolute prediction, never a measured
loop. It is accepted only for the exact admitted deck/deckId/play session,
the source `pedal-no-response-predicted`, a response window from 50 through 1500 ms,
one monotonic `pedalIntentId`, the exact `baseMeasuredLoopRevision` and
`baseLoopDivision`, and one exact next value in the full `8` through `1/64`
downward profile. The receiver verifies the entire causal base before the engine
call, so an older prediction cannot overwrite a later fresh measurement. It does not
advance measured-loop revision authority; a later fresh `DJ_LOOP_STATE` overrides
it. It is rejected after Release and cannot be synthesized from invalid, stale, or
contradictory measurements.

`DJ_RELEASE` turns the admitted owner's current DJ loop OFF and relinquishes the
DJ clock only when the Timeline is already playing. It does not resume, start,
seek, jump, or otherwise change Timeline position, playing state, child
transport, or Follow; the local clock naturally continues from the current
playhead. Exact replay and repeated Release are idempotent and do not
double-advance, double-cue, or seek.

`DJ_STATE_SYNC` restores diagnostics only. A snapshot with `released: true` cannot
re-enable or resume the loop. State Sync never replays Track Active mappings,
converges measured-loop authority, or executes Release transport semantics.
An empty restarted receiver accepts a structurally valid generic owner triple as
diagnostic evidence without manufacturing ownership; a receiver that already has an
owner requires an exact triple match. Only the later re-announced `DJ_TRACK_ACTIVE`
may run mapping admission on the restarted receiver.

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
- admitted owner deck/session, playing state, current exact identity, measured Loop state;
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
4. content-ID priority, exact title+artist fallback, title-only/non-playing/no-mapping
   rejection, non-Master actual-play acceptance, concurrent-owner protection, and
   Once-per-deck/play-session dedupe;
5. Track Load and State Sync never triggering a Timeline;
6. measured-loop revision/source freshness, non-accumulation, no-op convergence,
   exact nested active/inactive shape, retired-flat rejection, invalid
   authored-region/bounds rejection, monotonic intent/base causality, and bounded
   no-response fallback across the full `8` through `1/64` profile;
7. Release replay turns the admitted DJ loop OFF and, only when the Timeline is
   already playing, relinquishes DJ clock authority once, without changing
   transport, position, playing state, child transport, or Follow, and without a
   duplicate cue;
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

- Historical rb-output v1.1.6 runtime source
  `ee2f6c3148f36dfd63e0b70e2ab372247dbb8572` passed full `npm test`: 406
  tests / 404 pass / 0 fail / 2 intentional real-packaging-only skips. Focused
  envelope, Stage 1, smoke, config/setup, and CSV gates also passed, and the
  first-party warning count was 0. Independent Terra xHigh adversarial review
  found P0/P1/P2 none after the any-deck owner, frozen identity, reconnect
  reannounce, measured-loop, and admitted-owner UI corrections.
  This is dated software evidence only; it is superseded by the completed v1.1.8
  peer checkpoint and does not alter the hardware boundary. Ox-alpha was unavailable
  in that session; the narrow review exception is retained as historical context.

- Historical KDMX alpha.16 tranche: with the exact MSVC 14.44 linker pinned and
  first in `where.exe link.exe`, the final alpha.16 tranche passed sender-contract 6/6, the capability-family I/O
  focus, both extracted Syndocal any-deck runtime tests, and `cargo check` with
  first-party warnings 0. `check:dj-link`, localization 3555/3555, the
  three-display alpha.16 harness 80/80, frontend build, release metadata in a
  clean detached worktree, fmt, and diff checks passed. The
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

### SUPERSEDED / HISTORICAL — 2026-08-26 alpha.15 FOH/DJ transport preflight

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

### SUPERSEDED / HISTORICAL — 2026-08-26 alpha.15 live listener activation

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

### SUPERSEDED / HISTORICAL — 2026-08-26 strict-v3 initial snapshot and Timeline mapping-selection checkpoint (alpha.15/alpha.16)

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
video evidence. It demonstrates Track pre-load without trigger, exact mapped playback
on a non-Master or Master deck, concurrent-deck ownership fencing, repeated absolute
measured-loop reports, filter isolation, Release,
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
| [ ] HW-4.2 | Track pre-load, preview, Cue, non-playing, nonmatching, and ambiguous identity do not trigger | Required / Peer and hardware pending |
| [ ] HW-4.3 | Actual exact mapped playback on any deck emits one `DJ_TRACK_ACTIVE` and starts one mapped Timeline | Required / Peer and hardware pending |
| [ ] HW-4.4 | Concurrent playing decks cannot steal an admitted owner; terminal release permits a later mapped deck/session | Required / Peer and hardware pending |
| [ ] HW-4.5 | Stage 1 F14 local LoopHalf plus repeated absolute measured-loop `DJ_LOOP_STATE` reports | Required / Peer and hardware pending |
| [ ] HW-4.6 | Current v1.1.8 Stage 1 F13: HPF CC16 start plus immediate exactly-once `DJ_RELEASE`, then ChannelFader CC17 fade, Cue/Stop, HPF/fader reset; local MIDI failures do not gate Release | Required / Peer and hardware pending |
| [ ] HW-4.7 | Stage 1 F13 Release, ACK/rejection/timeout, and retry disposition | Required / Peer and hardware pending |
| [ ] HW-4.8 | Stage 2 authoritative `running`; F13/F15 `-4/+4`, F14 absolute loop set, and no MIDI | Required / Peer and hardware pending |
| [ ] HW-4.9 | Disconnect/local Stage 1 operation, reconnect State Sync, and Stage 2 fail-closed behavior | Required / Peer and hardware pending |
| [ ] HW-4.10 | Same-session event dedupe and replay safety | Required / Peer and hardware pending |
| [ ] HW-4.11 | App restart and next-show reuse | Required / Peer and hardware pending |
| [ ] HW-4.12 | Art-Net/sACN traffic sharing the wired network during the DJ run | Required / Peer and hardware pending |

The separately developed DJ-Link peer has no current-final published release. The
immutable v1.1.3 package is blocked by its `DJ_MASTER_CHANGED` mismatch. The
required peer is branch `beta-v1.1.2`, package version `1.1.8`, clean and
upstream-equal at `0f3e8c6851857c8542c132a89a7d44289002b1f5`; its latest
non-Master Deck 2 router-to-real-MIDI seven-byte proof is focused `12/12` and
its stable suite is `415` total / `413` pass / `0` fail / `2` intentional skips.
For the 2026-08-30 performance, with preparation
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
exact mapped any-deck playback and owner fencing, absolute Loop divisions, Release, disconnect
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

## SUPERSEDED / HISTORICAL — KDMX runtime/operator preflight gaps

These dated pre-alpha.17 gaps are retained as historical context only; the
then-current alpha.17 source checkpoint superseded them. The current alpha.18
source authority is recorded at the end. **P1:** Web Remote/DJ Link enabled state, bind
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

This is the current source-contract wire; it is deliberately restated outside
every SUPERSEDED / DO NOT EXECUTE label. The controlled DJ-PC peer route is
exclusively `syndocal-envelope-v3`.
`generic-json`, `syndocal-envelope-v1`, and `syndocal-envelope-v2` are retired and must be rejected
explicitly without fallback, aliases, diagnostic selections, or implicit
conversion. The exact v3 frame is
`{v:3,type,agentId,sessionId,sequence,eventId,payload}`. An authenticated
session becomes ready only after `DJ_AGENT_HELLO`, an authoritative
`DJ_STATE_SYNC`, `DJ_TIMELINE_STATE_REQUEST`, and the corresponding canonical
timeline-state response. Missing, unknown, stale, reordered, or legacy-shaped
frames fail closed without fallback or implicit conversion.

The show peer advertises exactly the generic any-deck nine-capability set defined
in section 2. Master state has no ingress or trigger authority. The clean break
is complete in the controlled KDMX source: old capability sets, Master events,
and Master-shaped payload fields reject fail-closed while the generic HELLO and
mapped any-deck route remain the only positive software contract. This does not
claim target-DJ-PC deployment or change the HW-4 matrix, which remains **0/12
checked (0%)**.

This restatement moves the same authority out of the historical 2026-08-25
correction record below, which stays under its SUPERSEDED label as history
only. No HW-4 row is changed by this restatement: the matrix remains **0/12
checked (0%)**, every row remains `Required / Peer and hardware pending`, and
the whole-product accepted denominator remains **19/71 (26.8%)**.

## SUPERSEDED — historical v1.1.6 controlled-source preflight

Do not execute this dated subsection. Use **Current v1.1.8 controlled-source
authority — 2026-08-27** and the current operator runbook below instead.

The next acceptance action uses only the clean, upstream-equal target-DJ-PC source
checkout on `beta-v1.1.2` at docs tip
`789f7724a699324cd87171ef835b69486bcd4e70`; its exact strict-v3 runtime source
is `ee2f6c3148f36dfd63e0b70e2ab372247dbb8572`, source version `1.1.6`. Do not install the blocked
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

## SUPERSEDED / HISTORICAL — 2026-08-27 HW-4.11 machine-authority source checkpoint

This append-only section records the source checkpoint that superseded the two
historical implementation gaps above. The alpha.17 checkpoint was current at
that historical point; the current alpha.18 source authority is recorded at the
end. This section does not rewrite the dated hardware observations above and
checks no HW-4 row.

- **Old path:** listener enable/bind/start state and the process-local token did
  not survive a Syndocal restart; discovery exposed address-only candidates and
  could not prove the selected physical Show-LAN adapter. The retired
  `list_show_lan_interfaces` route is removed.
- **New path:** commit `dcf6e524eddfaf79a54856af458efef202c079e1`
  persists only V2 non-secret machine authority in
  `dj-link-machine-settings.json`. The 32-byte secret is stored separately in
  Windows Credential Manager targets
  `jp.seraf.ktn.syndocal/dj-link/v1` and the transaction-only
  `jp.seraf.ktn.syndocal/dj-link/v1/rollback`. Startup restores an explicitly
  armed authority only after a stable two-pass NLM observation and a fresh exact
  `(network GUID, adapter GUID, IPv4)` revalidation. The same sole
  `RemoteWsServer` is then started in DJ-only mode; Web Remote is not silently
  enabled and a generic Web-only Start cannot replace an armed DJ authority.
- **Crash and clean-break behavior:** a monotonic credential-generation high
  water mark prevents generation reuse. Prepare/commit/final-write failures
  restore only a separately verified prior credential and non-secret preimage;
  invalid, corrupt, future, V1, stale, ambiguous, incomplete, or unsupported
  state remains visibly fail-closed. Disarm writes an unarmed durable cleanup
  marker before revoking both credential slots and retries unfinished cleanup
  at restart. No compatibility fallback or hidden second listener exists.
- **Secret boundary:** ordinary status, persisted JSON, URLs, logs, renderer
  storage, and `Debug` omit or redact the token. After HELLO authentication the
  parsed bearer and JSON value are zeroized and replaced by a fixed valid
  non-secret sentinel before canonical/session retention. Arm/rotate exposes a
  token only in the operator-requested show-once renderer field for at most 30
  seconds; copy and unmount clear it. Clipboard history and operator-managed
  external configuration remain outside the application boundary.
- **Source proof:** exact MSVC 14.44 linker preflight was printed and verified
  before each Rust gate. `cargo check -p syndocal --no-default-features
  --locked` passed; focused tests passed Syndocal `119/119`, I/O `38/38`,
  protocol `13/13`, and control-plane freeze `1/1` over exactly `482` routes.
  First-party warnings were `0`. Frontend build passed; DJ Link, output-control,
  invoke `422`, routing `130/31/30/411`, localization `3560/3560`, five-size
  Remote disclosure scroll, and Timeline context-menu/performance checks passed.
  Independent Terra xHigh review approved with no blocking P0/P1/P2 finding.
- **Historical live-artifact boundary (2026-08-27):** the operator's untouched
  process is PID `46120`, exact path
  `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`, version
  `1.2.0-alpha.15`, size `58,523,648`, SHA-256
  `42D7B5AEECD520855D4645DF6178E2DE617EC1B7D794A3010750DE94912BE33E`.
  It remains responsive with `192.168.50.1:9100` LISTEN and the DJ-PC peer
  `192.168.50.2:58211` ESTABLISHED. It is historical alpha.15 runtime evidence,
  not an alpha.16 native acceptance artifact. No native build, restart, window
  QA, real CredMan/NLM restart, or target-DJ-PC deployment was performed.
- The DJ PC therefore remains deployed on rb-output `1.1.5`; source `1.1.6` is
  pushed but not deployed. Real restart/token reuse/reconnect/next-show proof is
  still pending. HW-4.1 through HW-4.12 remain exactly **0/12 checked (0%)** and
  `Required / Peer and hardware pending`.

## SUPERSEDED — historical v1.1.6 HW-4 companion snapshot

Do not execute this dated subsection. It is superseded by **Current v1.1.8
controlled-source authority — 2026-08-27** and the updated operator runbook.

The concise operator sequence is
`qa/DJ_HW4_OPERATOR_RUNBOOK_2026-08-27.md`. It resolves the target DJ checkout
instead of hard-coding a user profile, then fails closed unless branch
`beta-v1.1.2`, exact clean/upstream-equal docs tip
`789f7724a699324cd87171ef835b69486bcd4e70`, runtime ancestor
`ee2f6c3148f36dfd63e0b70e2ab372247dbb8572`, and source version `1.1.6` all
match. It records no token bytes and keeps native alpha.16 build identity,
strict-v3 authentication, physical observations, and bounded protocol/fault
injection distinct.

For that historical controlled v1.1.6 peer, HW-4.6 was fixed to
`releaseMacro.enabled=false`: F13 must prove direct local Rekordbox Stop and an
independently routed Syndocal Release. Filter-then-fade is not an executable
choice for this matrix; a disabled template `sequence` field is inert and must
not be edited or enabled. Changing that policy requires a separately implemented,
reviewed, and deployed peer change. The companion additionally records the
target-checkout path/SHA-256 of
`server/public/setup/CustomMIDI1-Syndocal-v1.1.6.csv`, operator confirmation that
it is applied in Rekordbox, the exact `CustomMIDI1` port, and F13/F14 expected
targets; launcher preflight does not prove those Rekordbox steps. The document
itself is no hardware evidence and changes no checkbox: HW-4 remains exactly
**0/12 checked (0%)**.

## SUPERSEDED / HISTORICAL — rb-output v1.1.7 controlled-source authority — 2026-08-27

This historical section superseded every earlier v1.1.6 operational instruction
in this document without rewriting the checkpoint record. At that checkpoint,
the then-current DJ-PC source authority was clean, upstream-equal peer commit
`2577496767cf4ca8c8abdcadddbb891c7a609a32` on `beta-v1.1.2`, package version
`1.1.7`, external configuration
`C:\SyndocalShow\dj-agent-v1.1.7.json`, and mapping artifact
`server/public/setup/CustomMIDI1-Syndocal-v1.1.7.csv`. Historical v1.1.5 and
v1.1.6 configurations/mappings are unsupported for current acceptance.

The then-current Stage 1 F13 contract was exactly `filter-then-stop` with
`releaseMacro.enabled=true` and `releaseFade.enabled=false`: on the admitted
owner channel, HPF CC16 ramps `64 -> 127` over `1000 ms` with `50 ms` updates;
the planned completion sends Cue/Stop Note37 exactly once; one correlated
`DJ_RELEASE` is routed independently of Filter/Stop success; and a best-effort
CC16 reset to `64` follows Release. No channel-fader/fade MIDI is reachable.
F14 uses measured Rekordbox loop authority through
`8 -> 4 -> 2 -> 1 -> 1/2 -> 1/4 -> 1/8 -> 1/16 -> 1/32 -> 1/64`; prediction is
allowed only after a true no-response boundary. Stage 2 remains Timeline-only:
F13/F15 perform `-4/+4`, F14 sets the absolute Timeline loop, and all three emit
zero Rekordbox MIDI.

MASTER is diagnostic only. One exact mapped track that is actually playing on
any intended deck, including a non-Master deck, is eligible for admission.
MASTER changes must not synthesize, replace, or retrigger the admitted owner.
The then-current Syndocal Release/Timeline behavior was not conditioned on local
Rekordbox Stop success.

Peer source proof passed a fresh full `node --test` run: `406` tests total,
`404` passed, `0` failed, and `2` REAL-package opt-in tests skipped. Staged
JavaScript syntax was `30/30`, focused runtime/release/config/launcher proof was
`17/17`, `git diff --check` passed, and independent adversarial review found no
P0/P1/P2 issue. Release packaging remains deliberately unclaimed because the
versioned v1.1.7 distribution artifacts have not been generated. These source
results close no physical row: HW-4 remains exactly **0/12 checked (0%)** until
the ordered hardware and bounded fault cases below are captured.

## Current v1.1.8 controlled-source authority — 2026-08-27

This section supersedes the historical v1.1.7 section above and every earlier
operational peer instruction in this document. The target-DJ-PC source must be
branch `beta-v1.1.2`, clean and upstream-equal at exact commit
`0f3e8c6851857c8542c132a89a7d44289002b1f5`, package version `1.1.8`,
with external show configuration
`C:\SyndocalShow\dj-agent-v1.1.8.json`, mapping artifact
`server/public/setup/CustomMIDI1-Syndocal-v1.1.8.csv`, and exact adapter
`syndocal-envelope-v3`. The latest non-Master Deck 2 router-to-real-MIDI
seven-byte proof passed focused `12/12`; the full peer suite passed `415` total /
`413` pass / `0` fail / `2` intentional skips. Production remains on version
`1.1.8`; this peer commit is pushed, clean, and independently reviewed GO.
v1.1.7 and older configurations/mappings are historical only and must not be
substituted.

The accepted Stage 1 F13 edge begins HPF CC16 and synchronously routes exactly
one correlated `DJ_RELEASE` before local MIDI completion. Syndocal's Release
leg turns the admitted DJ loop OFF and relinquishes DJ clock authority when
the Timeline is already playing; it does not seek, jump, start, or change
Timeline position, playing state, child transport, or Follow. The local DJ leg
then independently completes HPF CC16 `64 -> 127`, ChannelFader CC17 `127 -> 0`,
and Cue/Stop Note37, each ramped over `1000 ms` with `50 ms` updates, followed by
HPF `64` and fader `127` resets. Any local MIDI failure remains visible but
never suppresses the already-routed Release. Release replay is idempotent.

F14 retains the full measured loop profile through `1/64`; prediction is
allowed only after a true no-response boundary. The existing Stage 2
`timeline-control` boundary remains F13/F15 `DJ_TIMELINE_BEAT_JUMP` `-4/+4`,
F14 absolute Timeline loop, and zero Rekordbox MIDI. That Stage 2 beat-jump
boundary is unchanged and still requires direct confirmation; it is not the
Stage 1 Release path. MASTER remains diagnostic only; any exact mapped,
actually-playing deck may be admitted.

The last committed KDMX alpha.17 source checkpoint
`fb5d18fdf898a1435bed173ddd17934a04a97897` is pushed and upstream-equal. Its
evidence is engine `dj_link_` `25/25`,
Syndocal `dj_link_dispatch_` `8/8`, three-display harness `80/80`, frontend
build pass, detached `check:release` pass including `169` ASIO packaging
assertions, and `0` first-party warnings. The exact native artifact is
`C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`,
`1.2.0-alpha.17`, `59,021,824` bytes, SHA-256
`8B35A0F89ED6FA9A1BF8B1929BFA323F7F6250DF059D6314CCE7DDD6D39EBE45`,
PID `57640`, exactly one responsive maximized window. These are software/native
proof only; target-DJ-PC deployment and HW-4 remain **0/12 checked (0%)**.

### 2026-08-27 latest alpha.18 source/UI authority

The current KDMX source checkpoint is `1.2.0-alpha.18` at branch
`codex/syndocal-v1.2`, exact `HEAD`/upstream
`db4eefc348b01ee05dd2dc87945afa85de8803e`. The alpha.17 native
artifact/process identity and hash above remain historical and are not re-bound
to alpha.18. The required `pnpm --dir app tauri build --no-bundle` passed with
exact MSVC 14.44 linker-first setup and zero first-party warnings. The resulting
alpha.18 artifact is:

- path: `C:\Users\kouty\Documents\KDMX\target\release\syndocal.exe`
- Product/FileVersion: `1.2.0-alpha.18`
- size: `58,740,224` bytes
- SHA-256: `841068E08F80EB877FBA919FB86D3F52B3D4444314B47ABEF995BFA593E8D4F9`
- LastWriteTimeUtc: `2026-08-27T05:38:40.1840641Z`
- launch: exactly one responsive process, PID `80264`, title `Syndocal`, window handle `854080`, `IsMaximized=true`, start `2026-08-27T14:38:57.8350806+09:00`

The existing alpha.17 native artifact returned
`wired_candidate_discovery_failed` during wired refresh; its typed live
diagnostic exposed the old `Structural DuplicateIpv4Address` code. The root
cause was a typed `sin_addr` read against the `SOCKADDR_IN` `+8` padding. The
source fix corrects that layout handling and drops COM objects before
`CoUninitialize`. Exact MSVC/FFmpeg live-unit and hardware-enumeration checks
passed and identify Ethernet4 `192.168.50.1` as eligible. The native screenshot
confirmed Web Remote/Security/Endpoints/DJ Link/Standby in the same disclosure
stack and showed two wired candidates. The attempted native wired-binding
refresh click remains unconfirmed because foreground PID retrieval failed. The
five-viewport setup harness independently confirmed listener empty→count `0`→
Ethernet4 `192.168.50.1`→count `1`, one invoke per phase, and disabled mutation
controls; this does not promote the native click to accepted.

The Web Remote source layout now uses the same connection disclosure stack as
DJ Link and Endpoints; controls were not resized. Native visual confirmation is
verified above; only the direct native refresh-button click remains unconfirmed.
The standard and dedicated Setup I/O browser contracts pass all
five viewports, including `1280x720`. Independent review found and closed an
adjacent fail-closed defect where rejected DMX network-route buttons mutated the
protocol draft before reporting `no state changed`; the candidate-only path now
leaves the full draft and all `128` route signatures unchanged and invokes no
retired output command. Remote Start/Stop is asserted exactly once and in order;
independent re-review is GO with P0/P1/P2 all zero. Focused root revalidation of the MASTER clean break passed
protocol `7/7`, runtime `5/5`, I/O `37/37`, frontend/build, live, and static
checks under the required single-thread standard gate; independent review is GO.
A parallel I/O race is baseline-existing and is not acceptance evidence;
the exact MSVC 14.44 / locked full-workspace rerun passed `1164` pass / `0` fail
/ `11` intentional hardware-media ignores in the Syndocal binary target with
zero first-party warnings. The touched full-gate fixture repairs were
independently reviewed GO with P0/P1/P2 all zero; frontend invokes are `422`,
localization is `3559`, and the frontend production, viewport/setup, and warning
ratchets are green. The clean release gate `4fc443d` passed after staging seven
pinned DLLs. Main-checkout `check:release` remains intentionally blocked by the
tracked, unchanged runtime-inventory hard-link alias to
`C:\Users\kouty\Documents\.tmp.driveupload\867492`; no alias deletion or file
replacement was performed. Native build/launch/layout integration is verified;
the direct refresh-button action remains unconfirmed.

`app/dist` has already been freshly rebuilt and its old stale marker is `0`.
The ignored peer `dist` remains stale at `277,382,202` bytes but is outside the
production/source checkpoint. Cleanup of the prior alpha.17 temporary tree
(`394,438,512` bytes) remains policy-blocked with no bytes reclaimed. The
reviewer baseline
`C:\Users\kouty\AppData\Local\Temp\kdmx-head-baseline-review-20260827-1246`
is now `49,009,359` bytes after `885.6 MiB` was reclaimed by `cargo clean`;
direct cleanup remains policy-blocked. The DJ/Pedal matrix remains **0/12
checked (0%)**; none of this continuation is native or physical acceptance.

### 2026-08-27 production title-selector editor source checkpoint

Branch `codex/syndocal-v1.2` advances from exact upstream-equal
`129d08d76141e7e69363a454d429bc3acca6b50e`. The source editor can now
author `titleContains = 人生オーバー` with explicit `fallbackDeck = 1`, edit
and display it, and preserve it through the project-control mapping path.
`Use Current Track` remains exact. A dedicated pure policy outside `App.tsx`
rejects mixed, duplicate, over-limit, invalid-ID, over-256-byte, or
control-character selectors and keeps a rejected editor draft visible.

Focused DJ Link, localization **3564/3564**, TypeScript, production build, and
all five Setup I/O viewport gates passed with zero first-party warnings.
Independent Terra xHigh review is GO after the local-error/draft-retention P1
was fixed. This is source/UI evidence only: no alpha.19 native artifact exists,
the running alpha.18 binary does not contain it, and HW-4 stays **0/12**.
