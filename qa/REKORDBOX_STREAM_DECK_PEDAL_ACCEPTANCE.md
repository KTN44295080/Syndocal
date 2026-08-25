# Syndocal × rekordbox-DJ-Link-ForPCDJ acceptance

Date: 2026-08-21
Status: Required; Syndocal implementation in progress; DJ-Link peer implementation and hardware acceptance pending
Source authority: replacement user specifications received 2026-08-20 and 2026-08-21

Current-train notice (updated 2026-08-25): the active product train is
`1.2.0-alpha.11`; the alpha.10 pause was rescinded before promotion. Follow
`AGENTS.md` and `qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md` for current work.
This file remains the detailed DJ/pedal gate. The alpha.8 references in the
2026-08-24 checkpoint below are historical evidence only, not a claim that
alpha.8 is the current Syndocal artifact or authority. The live acceptance status
remains `Required / Peer and hardware pending`. The 2026-08-25 updated peer
source/package audit is recorded in its own section below.

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

Every DJ frame is strict, bounded JSON. The `syndocal-envelope-v1` adapter uses the
envelope shape below; the explicit `generic-json` adapter carries the corresponding
semantic fields at the frame root and uses its fixed flat ACK shape:

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

`DJ_AGENT_HELLO` is the first Agent-to-Syndocal frame and carries a dedicated token,
Agent version, and capabilities. The token is backend-generated, shown only through
an explicit rotation flow, and is never placed in a URL, query string, ordinary status
response, `.sdc`, template, backup, or Standby checkpoint. After HELLO, an
authenticated Agent session may send only these Agent-to-Syndocal event types:

- `DJ_HEARTBEAT`
- `DJ_MASTER_CHANGED`
- `DJ_MASTER_TRACK_ACTIVE`
- `DJ_LOOP_STATE`
- `DJ_RELEASE`
- `DJ_STATE_SYNC`
- `DJ_TIMELINE_STATE_REQUEST` (control frame with an empty payload)
- `DJ_TIMELINE_BEAT_JUMP`
- `DJ_TIMELINE_LOOP_SET`

On the production `syndocal-envelope-v1` wire, the Syndocal-to-Agent direction has
one authoritative timeline-state event, `DJ_TIMELINE_STATE`, plus ACK responses to
admitted physical events. The explicit `generic-json` compatibility adapter also
accepts `DJ_STATE_SYNC_REQUEST` or its legacy `STATE_SYNC_REQUEST` alias and answers
with a fresh Agent-to-Syndocal `DJ_STATE_SYNC`; the Syndocal production listener does
not emit either compatibility request. The Agent never originates
`DJ_TIMELINE_STATE`; it only decodes that event and changes mode from its authoritative
contents.

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
isPlaying }`. `DJ_TIMELINE_STATE_REQUEST` has no payload. `DJ_TIMELINE_BEAT_JUMP`
carries `{ bars: -4|4, timelineId }`, and `DJ_TIMELINE_LOOP_SET` carries
`{ active: boolean, timelineId }`; both are Agent-to-Syndocal and ACKed.

`DJ_TIMELINE_STATE` is Syndocal-to-Agent and authoritative. Its semantic fields are
`state` (`idle`, `running`, `stopped`, `ended`, or `reset`), boolean `loopActive`,
`timelineId`, and nonnegative `positionBars`, together with the event identity and
sequence. The generic-json adapter places those fields at the frame root; the
`syndocal-envelope-v1` adapter places them under the envelope `payload`. State Sync
updates diagnostics and absolute Loop truth only; it cannot fire a Track mapping or
infer Release. Stage 2 F14 derives its absolute `active` value as the logical inverse
of the latest authoritative `DJ_TIMELINE_STATE.loopActive` for that `timelineId`;
neither the local LoopHalf counter nor an ACK alone is an authority update.

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

In Stage 1, F13 runs the configured local release macro: Filter HP and the master
deck's `ChannelFader` fade, Cue/Stop, optional local reset steps, and then one
idempotent `DJ_RELEASE`. F14 keeps the local MIDI LoopHalf action and
sends `DJ_LOOP_STATE` with the absolute division. F15 is deliberately inactive in
Stage 1 and sends neither MIDI nor a Syndocal show event. F13 macro ordering is
configuration-dependent: the documented default is `sequence:"parallel"`, while
`filter-then-fade` waits for Filter completion before starting the fade. Ramp
duration/interval and reset-after-stop policy come from the peer configuration; a
ramp or reset failure does not advance to Stop/Release. With the macro disabled,
the legacy direct Stop/Release path is the fallback.

Only an authoritative `DJ_TIMELINE_STATE` with `state:"running"` enters Stage 2.
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
operation during disconnect, reconnect State Sync, and ACK display. The current
`C:\Users\kouty\Desktop\rb-output` source was statically inspected on
`beta-v1.1.2`: `tests/smoke.test.js` contains 60 test declarations and
`tests/syndocal-envelope-v1.test.js` contains 9. The following focused declarations
are the existing source evidence for the Stage 2 and reconnect contract; this is a
static test inventory, not a hardware execution claim:

- `tests/smoke.test.js`:
  `timeline-control maps pedals to ACKed timeline actions without MIDI and fails
  closed on disconnect`; `Syndocal disconnect does not gate Stage 1 local MIDI
  actions`; `release handoff failures never stick in handoff-pending and running
  wins the late-failure race`; `every physical event waits for typed ACK outcomes,
  including master and timeline events`; `invalid State Sync snapshots never send
  or request timeline, then recover on reconnect`; and `Busy backoff is fenced to
  its socket and reconnect never replays old events` (6 declarations).
- `tests/syndocal-envelope-v1.test.js`:
  `syndocal-envelope-v1 physical events use exact typed payloads and strict ACK
  semantics`; `syndocal-envelope-v1 rejects malformed outbound payloads fail-closed
  and decodes only valid timeline states`; and `Stage 1 local MIDI independence and
  Stage 2 network fail-closed gates hold on syndocal-envelope-v1` (3 declarations).

These tests statically cover the authoritative `running` gate, F13/F15 `-4/+4`
beat-jump payloads, F14 absolute loop-set payload, no Stage 2 MIDI, invalid/missing
state handling, typed ACK rejection/timeout, and disconnect fail-closed behavior.
They do not close the physical pedal, rekordbox, wired-LAN, or two-process rows.

## 9. Native and hardware acceptance

End-to-end acceptance records both repository commits/artifacts, Windows and app
versions, DJ/FOH NICs and switch path, rekordbox and Stream Deck versions, Pedal model
and firmware, virtual MIDI device/mapping, operator/date, packet/log timestamps, and
video evidence. It demonstrates Track pre-load without trigger, actual Master playback
trigger, Master switch, absolute repeated Loop divisions, filter isolation, Release,
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
| [ ] HW-4.5 | Stage 1 F14 local LoopHalf plus absolute/repeated `DJ_LOOP_STATE` divisions | Required / Peer and hardware pending |
| [ ] HW-4.6 | Stage 1 F13 Filter isolation and configured local release-macro behavior | Required / Peer and hardware pending |
| [ ] HW-4.7 | Stage 1 F13 Release, ACK/rejection/timeout, and retry disposition | Required / Peer and hardware pending |
| [ ] HW-4.8 | Stage 2 authoritative `running`; F13/F15 `-4/+4`, F14 absolute loop set, and no MIDI | Required / Peer and hardware pending |
| [ ] HW-4.9 | Disconnect/local Stage 1 operation, reconnect State Sync, and Stage 2 fail-closed behavior | Required / Peer and hardware pending |
| [ ] HW-4.10 | Same-session event dedupe and replay safety | Required / Peer and hardware pending |
| [ ] HW-4.11 | App restart and next-show reuse | Required / Peer and hardware pending |
| [ ] HW-4.12 | Art-Net/sACN traffic sharing the wired network during the DJ run | Required / Peer and hardware pending |

Until the separately developed DJ-Link peer exposes the fixed contract and both builds
pass the wired-LAN hardware matrix, this feature remains `Required / Peer and hardware
pending`; Syndocal-side automated completion is not an end-to-end completion claim.

## 2026-08-24 software/package checkpoint (historical alpha.8-era evidence)

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

## 2026-08-25 peer v1.1.1 historical unbound software/package smoke

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

## 2026-08-25 updated DJ-Link peer audit (peer source/package checkpoint)

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
