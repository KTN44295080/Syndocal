# Syndocal × rekordbox-DJ-Link-ForPCDJ acceptance

Date: 2026-08-21
Status: Required; Syndocal implementation in progress; DJ-Link peer implementation and hardware acceptance pending
Source authority: replacement user specifications received 2026-08-20 and 2026-08-21

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

## 2. Shared DJ Link wire contract

DJ-Link is the WebSocket client. Syndocal extends its existing Web Remote listener with
the dedicated `/dj-link` role/path; it does not open an unrelated second server.
Generic Remote authorization and DJ Link authorization remain separate.

Every DJ frame is a strict, bounded JSON envelope:

```json
{
  "v": 1,
  "type": "DJ_MASTER_TRACK_ACTIVE",
  "agentId": "stable-agent-id",
  "sessionId": "connection-session-id",
  "sequence": 104,
  "eventId": "opaque-event-id",
  "payload": {}
}
```

`DJ_AGENT_HELLO` is the first frame and carries a dedicated token, Agent version, and
capabilities. The token is backend-generated, shown only through an explicit rotation
flow, and is never placed in a URL, query string, ordinary status response, `.sdc`,
template, backup, or Standby checkpoint. Only an authenticated session may send:

- `DJ_HEARTBEAT`
- `DJ_MASTER_CHANGED`
- `DJ_MASTER_TRACK_ACTIVE`
- `DJ_LOOP_STATE`
- `DJ_RELEASE`
- `DJ_STATE_SYNC`

Sequence is a positive JavaScript-safe integer and increases monotonically for one
`agentId`/`sessionId`; gaps are allowed. `eventId` is opaque and bounded. Same ID plus
the same canonical shape returns the saved terminal response without repeating a side
effect. Same ID with another shape, sequence rollback, unauthenticated traffic, and
session impersonation are rejected. A newly authenticated session replaces an older
session by generation; an old socket closing cannot clear the new session.

Acknowledgement includes the original ID and sequence:

```json
{
  "v": 1,
  "type": "ACK",
  "eventId": "opaque-event-id",
  "sequence": 104,
  "outcome": "accepted",
  "code": "ok",
  "stateGeneration": 42
}
```

Allowed outcomes are `accepted`, `duplicate`, `no_mapping`, `rejected`, and `busy`.
`busy` is explicitly nonterminal: the peer retains the exact event identity and may
retry it, while the other admitted outcomes are cached for idempotent replay. An ACK
is sent only after validation and canonical admission, and `accepted` is returned only
after the canonical engine lane reports the semantic result. It never falsely claims
that an external physical action occurred. Heartbeat is approximately five seconds;
Syndocal marks the peer disconnected after a bounded timeout of approximately fifteen
seconds. Disconnect or timeout never implies Release.

The shared wire fixtures use the event names exactly as written above; implementations
must not shorten them to `HELLO`, `MASTER_TRACK_ACTIVE`, or similar private aliases.
`DJ_MASTER_TRACK_ACTIVE.payload` carries `deck`, `contentId`, `title`, `artist`,
`trackBpm`, `positionSec`, `startedAt`, and `playSessionId`. `DJ_LOOP_STATE.payload`
carries the absolute `division`. `DJ_STATE_SYNC.payload` carries `loopDivision`,
`released`, `masterDeck`, and nested `masterTrack { contentId, title, artist,
isPlaying }`. State Sync updates diagnostics and absolute Loop truth only; it cannot
fire a Track mapping or infer Release.

## 3. DJ-Link peer behavior

The peer reuses the existing hook, Hook UDP, master-change event, playback state,
Socket.IO/Web UI, packaging, and installer. Its new show-control client sends explicit
semantic events rather than the existing large browser `state` snapshot.
The peer repository is implemented and reviewed in its own Codex flow. Syndocal does
not modify that repository in this tranche; integration acceptance pins both immutable
repository commits once the peer checkpoint is available.

The Pedal defaults may use F13/F14/F15, but remain configurable and are acquired as
native Windows global hotkeys on the DJ PC, not through browser `keydown` and not by
Syndocal.

- Pedal 2 sends one configured local MIDI Loop Half action and advances the Agent's
  absolute `loopDivision`. It sends `DJ_LOOP_STATE` with the absolute division.
- Pedal 3 runs the configured nonblocking local MIDI CC ramp. It sends no Syndocal
  show event.
- Pedal 1 sends the configured local deterministic stop, optional local reset steps,
  and one idempotent `DJ_RELEASE` which is not complete until acknowledged.

Network loss does not block local rekordbox control. The peer displays the loss and,
after reconnecting, sends `DJ_STATE_SYNC` with current state instead of replaying old
relative actions.

## 4. Master Track Active event

Track Loaded, Track Playing, and Master Track Active are distinct. Automatic show
mapping uses only `DJ_MASTER_TRACK_ACTIVE`, generated when the deck is current Master,
is actually playing, has a known identity, and differs from the prior active play
session. It is also generated when an already-playing deck becomes Master.

The payload contains bounded `deck`, `contentId`, `title`, `artist`, `trackBpm`,
`positionSec`, `startedAt`, and `playSessionId`. An explicit hook master-change wins
over explicit master state, which wins over the existing playback heuristic.

## 5. Project Track-to-Timeline mapping

Syndocal stores `DJ Track Trigger Mapping` in `.sdc` because show outcome is project
data. The DJ-Link peer does not store it. A mapping contains:

- stable mapping ID;
- selector: exact opaque `contentId`, or exact normalized `title` plus `artist`;
- event: `MasterTrackActive`;
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

`DJ_MASTER_CHANGED` updates diagnostics only. Load, preview, Cue preparation, a
non-Master deck, and `DJ_STATE_SYNC` cannot start a Timeline.

`DJ_MASTER_TRACK_ACTIVE` validates current Master/playing/identity, resolves at most
one mapping, and invokes the existing canonical runtime Timeline start path. The
dedupe key includes project epoch, mapping ID, and `playSessionId`; mapping CAS or
project replacement invalidates the prior generation.

`DJ_LOOP_STATE` is absolute. Division zero restores the authored A-B duration;
division `N` sets the end to `A + (B-A)/2^N`. It uses the existing Loop runtime,
rejects missing regions, bounds and overflow, and is a no-op when already converged.
It must never implement absolute synchronization by repeatedly applying relative
Loop Half.

`DJ_RELEASE` disables the current DJ loop and resumes the Timeline through the
canonical transport lane. Exact replay and repeated Release are idempotent and do not
double-advance, double-cue, or seek.

`DJ_STATE_SYNC` restores diagnostics and may converge an explicit absolute loop
division only when the synchronized state is not released. A snapshot with
`released: true` cannot re-enable or resume the loop. State Sync never replays Track
Active mappings and never executes Release transport semantics from a snapshot.

## 7. Syndocal operator surface

The existing Web Remote/Setup I/O surface contains a `DJ Link agent` disclosure with:

- configured Show-LAN bind address and endpoint state;
- Connected/Disconnected, peer address, heartbeat age, and session generation;
- Master deck, playing state, current title/artist/content ID, Loop division;
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
6. absolute Loop division 0/1/N, non-accumulation, no-op convergence, and invalid
   authored-region/bounds rejection;
7. Release replay disabling/resuming once without seek or duplicate cue;
8. legacy project default, exact save/reload, mapping CAS conflict, template,
   backup/recovery, and Standby mapping round-trip;
9. project replacement or failed CAS producing no stale event side effect;
10. token/session/runtime state never appearing in project artifacts or ordinary
    status responses;
11. existing Remote, MIDI, OSC, WebSocket, Timeline, Video, Lighting, and warning
    ratchets remaining green.

The DJ-Link peer separately proves Hook/Now Playing regression safety, Master Track
Active generation, Pedal/global-hotkey input, local MIDI mappings/ramp/reset, local
operation during disconnect, reconnect State Sync, and ACK display.

## 9. Native and hardware acceptance

End-to-end acceptance records both repository commits/artifacts, Windows and app
versions, DJ/FOH NICs and switch path, rekordbox and Stream Deck versions, Pedal model
and firmware, virtual MIDI device/mapping, operator/date, packet/log timestamps, and
video evidence. It demonstrates Track pre-load without trigger, actual Master playback
trigger, Master switch, absolute repeated Loop divisions, filter isolation, Release,
disconnect/local operation/reconnect sync, same-session dedupe, app restart, and next
show reuse while Art-Net/sACN traffic shares the wired network.

Until the separately developed DJ-Link peer exposes the fixed contract and both builds
pass the wired-LAN hardware matrix, this feature remains `Required / Peer and hardware
pending`; Syndocal-side automated completion is not an end-to-end completion claim.

## 2026-08-24 software/package checkpoint

The separately developed DJ-Link peer package `1.1.0` is now committed and pushed
on `Beta` at `6c4f4328a6866d9d48022bd8ee20a7887c9de851`. Its package/runtime checks passed:
54 tests, 16 Node syntax checks, and zero warnings. The packaged
`C:\Users\kouty\Desktop\rb-output\dist\server.exe` is SHA-256
`339ECF6E82EB463F55977F63A137CB0CB52886CD7E2874E87F5AD4724234377B`. This
closes only the peer software/package checkpoint; it does not promote any
hardware or wired-LAN row.

On the current Windows machine, rekordbox was not running and the Stream Deck
Pedal was not detected during this checkpoint. The old peer process remained on
UDP 22346/HTTP 8787; no production cutover was claimed, and the known Daslight
and Ableton processes were left untouched. Syndocal's current alpha.8 native
build/reload and completed five-display pane-route evidence are recorded
separately in
`qa/artifacts/native-physical-acceptance/2026-08-24-alpha8-current-source-final/native-display-route-current-source.json`
(SHA-256 `41F1D6E2528E7439657F8879F753255221E25F2DF0474139B56C1570E2C32C41`);
neither substitutes for the DJ/rekordbox/State Sync/Pedal matrix.

The alpha.8 staged-source authority is
`qa/artifacts/source-freeze/2026-08-24-alpha8-current-source-freeze.sha256`, with
103 payload records and its own manifest envelope excluded to avoid recursive
self-hashing. The alpha.7 pre-alpha.8 manifest is historical only. The display
route accepts placement/maximize/containment only: all five pane captures still
show the owner-registration status at six seconds, so transactional pane
operation, warning-clean pane startup, and completed owner registration remain
unaccepted alpha.9 work.

Therefore the authoritative status remains `Required / Peer and hardware
pending`. Still-open evidence includes wired-LAN HELLO/auth/session replacement,
real Master playback and switch, absolute Loop divisions, Release, disconnect
and reconnect State Sync, Pedal/global-hotkey input, app restart/next-show reuse,
and concurrent Art-Net/sACN traffic. No DJ/Pedal completion claim is made and the
KDMX accepted denominator remains 19/71 (26.8%).
