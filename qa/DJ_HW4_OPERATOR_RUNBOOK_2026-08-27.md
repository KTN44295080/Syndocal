# DJ-Link HW-4 operator runbook — 2026-08-27

Status: **execution pending; HW-4 remains 0/12**

This is the short execution companion to
`qa/REKORDBOX_STREAM_DECK_PEDAL_ACCEPTANCE.md`. It does not replace that
acceptance authority and must not promote a row from a static gate, preflight,
socket, or `CONNECTED` label alone.

## 1. Stop boundary and source identity

Do not begin while the current live DJ session must remain connected. Obtain
explicit operator approval before replacing the live Syndocal process.

Immediately before the build:

1. Record KDMX branch, exact `HEAD`, upstream, and `git status --short`.
2. Resolve every process whose executable path is exactly this checkout's
   `target/release/syndocal.exe`.
3. Record PID, path, file size, Product/FileVersion, SHA-256, window title,
   responsiveness, and owned TCP sockets.
4. Force-terminate only the exact-checkout process. Do not stop Daslight or a
   `syndocal.exe` from another checkout.
5. Run `pnpm --dir app tauri build --no-bundle`. The tracked wrapper must log
   `vcvars64.bat -vcvars_ver=14.44`, the exact Community
   `14.44.35207\bin\Hostx64\x64\link.exe` Cargo pin, and that same linker first
   in `where.exe link.exe`. Any Git-linker-first attempt is invalid.
6. Record the newly built file identity, launch it, and verify exactly one
   responsive maximized `Syndocal` window before UI acceptance.

The build commit is whatever exact clean, upstream-equal KDMX `HEAD` is recorded
at execution time. Never substitute an older source hash or an alpha.19-or-earlier
process as the alpha.20 artifact identity.

### 1.1 Current alpha.20 / Follow / Stage 2 execution gate — 2026-08-27

KDMX `1.2.0-alpha.20` is the current source/native checkpoint; its authoritative
source is the commit containing this runbook update. The immutable Follow /
Stage 2 source authority remains alpha.19 commit
`41faefc054a3c37cef81cfd2e69b4e3f3df5ab4f` and is carried forward without a
wire or policy change.
The external peer is committed/pushed at
`b03d66a87b8d9dcdedfbd9b5c395bda7df7e0eec`, version `1.1.9`. Follow hold,
Stage 2 F13 loop-off, reconnect fail-closed behavior, focused/full suites, and
independent source review are complete. The local alpha.20 native build was
launched as exactly one responsive maximized window, but target-DJ-PC identity,
configuration, token, NIC, and physical observations are still required before
any row is accepted. HW-4 remains **0/12** until those observations are recorded;
source or local native acceptance alone closes no hardware row.

## 2. DJ-PC controlled source

Open one PowerShell inside the actual target-DJ-PC checkout. Resolve and verify
that checkout rather than copying a workstation-specific path:

```powershell
$PeerRoot = (git rev-parse --show-toplevel).Trim()
if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $PeerRoot)) {
  throw 'Current directory is not the target DJ-PC source checkout.'
}
Set-Location -LiteralPath $PeerRoot
$ExpectedBranch = 'beta-v1.1.2'
$ExpectedPeerHead = 'b03d66a87b8d9dcdedfbd9b5c395bda7df7e0eec'
$ExpectedPeerVersion = '1.1.9'
$branch = (git branch --show-current).Trim()
$dirty = @(git status --porcelain=v1)
$head = (git rev-parse HEAD).Trim()
$upstream = (git rev-parse '@{upstream}').Trim()
$version = (node -p "require('./package.json').version").Trim()
if ($branch -ne $ExpectedBranch -or $dirty.Count -ne 0 -or
    $head -ne $ExpectedPeerHead -or $upstream -ne $ExpectedPeerHead -or
    $version -ne $ExpectedPeerVersion) {
  throw 'DJ-PC checkout failed exact controlled-source identity validation.'
}
[pscustomobject]@{
  Root=$PeerRoot; Branch=$branch; Clean=($dirty.Count -eq 0)
  Head=$head; Upstream=$upstream; Version=$version
}
```

The required identity is a clean, upstream-equal peer commit at source version
`1.1.9`, exactly `b03d66a87b8d9dcdedfbd9b5c395bda7df7e0eec` for both `HEAD` and
`@{upstream}`, with strict adapter `syndocal-envelope-v3`. The final focused
envelope/smoke gate passed `115/115`; the full peer suite passed `455` total /
`453` pass / `0` fail / `2` intentional skips. The commit is pushed, clean, and
independently reviewed GO. This is source/full-suite evidence only and closes no
HW-4 row.
Branch name alone is insufficient. Do not use an installer, the blocked v1.1.3
release, or a historical v1.1.5/v1.1.6/v1.1.7/v1.1.8 configuration as current
acceptance evidence.

The current KDMX source/native checkpoint is `1.2.0-alpha.20` at branch
`codex/syndocal-v1.2`, in the commit containing this runbook update. The DJ
authority it carries is alpha.19 source commit
`41faefc054a3c37cef81cfd2e69b4e3f3df5ab4f`. At that alpha.19 checkpoint, the
exact MSVC 14.44 locked single-thread full workspace gate passed `2622` / `0`
failed / `15` intentional hardware-media ignores with zero first-party warnings,
and a clean detached non-OneDrive worktree passed
`pnpm --dir app run check:release` after staging the seven pinned runtime DLLs.
Those are historical alpha.19 results, not an alpha.20 release pass.

For alpha.20, frontend invokes are `422`, localization is `3582/3582`, and the
five-view Stage/Setup-I/O, stage labels, stage-live segments, TypeScript,
frontend production, and zero-warning gates passed. This active checkout's
tracked runtime-inventory hard-link alias still fails `check:release` before
metadata validation by design and was not deleted or replaced. The
alpha.20 `pnpm --dir app tauri build --no-bundle` artifact is Product/FileVersion
`1.2.0-alpha.20`, SHA-256
`E8100D160158034A63901EA1BF775EC06A48EFF4D97CAF454D0378C0D6988D7D`; it was
launched as exactly one responsive maximized window. Recheck that identity after
the checkpoint commit/push and immediately before HW-4 execution. Any
alpha.19-or-earlier artifact or PID is historical and must not be substituted
for alpha.20 acceptance.
The source UI now places Web Remote in the same
connection disclosure stack as DJ Link/Endpoints without shrinking controls;
the standard and dedicated Setup I/O browser contracts pass all five viewports,
including `1280x720`. An independent review found that rejected DMX network-route
buttons still changed the protocol draft before reporting `no state changed`;
the corrected candidate-only path now leaves the full draft and all `128` route
signatures unchanged, invokes neither retired output command, and independently
re-reviewed GO with P0/P1/P2 all zero. The earlier native visual capture belongs
only to the historical alpha.18 artifact; its direct native refresh-button click
also remains unconfirmed and neither observation accepts alpha.20. The current
alpha.20 native Stage/I/O visual proof is recorded in the main pause handoff;
native Web/DJ card switching remains unaccepted because UI Automation could not
act on the cached element.
`app/dist` is freshly rebuilt (old stale
marker `0`); the ignored peer `dist` remains stale but is outside this source
acceptance checkpoint. These observations do not promote native or HW-4
acceptance.

If and only if the v1.1.9 external configuration is absent:

```powershell
.\start-all.bat --init-config
```

Edit only `C:\SyndocalShow\dj-agent-v1.1.9.json`. Replace the one-time token
placeholder. Before preflight, record the versioned Rekordbox mapping artifact
from this exact target checkout; do not substitute a historical mapping:

```powershell
$MidiMappingPath = Join-Path $PeerRoot 'server\public\setup\CustomMIDI1-Syndocal-v1.1.9.csv'
if (-not (Test-Path -LiteralPath $MidiMappingPath -PathType Leaf)) {
  throw 'Missing current v1.1.9 Rekordbox CustomMIDI mapping artifact.'
}
$MidiMappingHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $MidiMappingPath).Hash
[pscustomobject]@{ Path=$MidiMappingPath; SHA256=$MidiMappingHash }
```

Record that path and SHA-256 in the row evidence, then obtain operator
confirmation that this exact CSV is applied in Rekordbox. Record the exact
`CustomMIDI1` port and expected targets: owner-channel CC16 controls HPF,
configured `stop` Note37 maps to Rekordbox Cue/Stop, and configured `loopHalf`
Note36 maps to Rekordbox LoopHalf. The launcher validates only the configured
device name and integer port; it does not apply or prove the Rekordbox CSV.
Then, in the same PowerShell:

```powershell
$ForbiddenOverrides = @(
  'DJ_AGENT_CONFIG','DJ_AGENT_ENABLED','DJ_AGENT_ALLOW_REMOTE_ACTIONS',
  'SYNDOCAL_ENABLED','SYNDOCAL_HOST','SYNDOCAL_PORT','SYNDOCAL_PATH',
  'SYNDOCAL_NIC','SYNDOCAL_TOKEN','SYNDOCAL_WS_ADAPTER',
  'SYNDOCAL_HEARTBEAT_MS','PEDAL_ENABLED','PEDAL_MODULE','MIDI_ENABLED',
  'MIDI_MODULE','MIDI_DEVICE','MIDI_PORT','MIDI_RELEASE_FADE',
  'MIDI_RELEASE_MACRO','MIDI_DECK_CHANNELS','PORT','RB_OUTPUT_HOST',
  'RB_OUTPUT_SETUP_MAPPING_PATH'
)
$PresentOverrides = @(Get-ChildItem Env: | Where-Object {
  $ForbiddenOverrides -contains $_.Name.ToUpperInvariant()
} | Select-Object -ExpandProperty Name)
if ($PresentOverrides.Count -ne 0) {
  throw "Remove forbidden show overrides from this PowerShell first: $($PresentOverrides -join ', ')"
}
$env:DJ_AGENT_CONFIG_PATH = 'C:\SyndocalShow\dj-agent-v1.1.9.json'
.\start-all.bat --preflight-only
.\start-all.bat
```

`--preflight-only` starts no show process and closes no HW-4 row. The
no-argument command is the real controlled-source launch.

Never paste the token into chat, logs, screenshots, commands, project files, or
evidence JSON. Record only `tokenConfigured: true`, length range validity, and
the resulting non-secret generation/session identifiers.

## 3. Authenticated baseline

On the maximized Syndocal window:

1. Open I/O > DJ Link and refresh physical network candidates.
2. Select the exact Ethernet candidate by network GUID, adapter GUID, and
   `192.168.50.1`; reject stale, virtual, ambiguous, or address-only identity.
3. Arm DJ Link only while the shared Remote listener is stopped.
4. Rotate once, copy the show-once token directly into the external DJ-PC JSON,
   and start the shared listener and DJ Agent without another intervening
   Syndocal restart.
5. Record `.50.1:9100`, peer `.50.2`, settings/credential generation, session,
   heartbeat, exact adapter, and cleanup/block state.
6. Capture real strict-v3 `HELLO -> ACK -> DJ_STATE_SYNC`, followed by an empty
   `DJ_TIMELINE_STATE_REQUEST -> authoritative DJ_TIMELINE_STATE` exchange.

Do not proceed from UI `CONNECTED` alone. Timeline state must be known and the
initial snapshot must be authoritative.

## 4. Evidence record

For every row record:

- date/time with time zone, operator, KDMX commit/artifact hash, peer commit and
  version, Rekordbox version, pedal model/firmware, virtual MIDI mapping;
- FOH/DJ NIC names, GUIDs, IPv4 addresses, switch path, and concurrent lighting
  traffic state;
- starting generation/session/deck/track identity and Timeline mapping;
- redacted packet/log timestamps, visible UI result, MIDI monitor result, video
  filename, and pass/fail reason;
- whether the observation came from normal UI/hardware or the required bounded
  protocol/fault harness.

A row passes only when every subcheck below has direct evidence. Otherwise leave
the checkbox open and name the missing subcheck.

## 5. HW-4 ordered matrix

- [ ] **HW-4.1 — authentication/session replacement.** Prove wired HELLO/auth,
  fresh generation and State Sync. Old-close/ABA protection requires an explicit
  two-session protocol/fault test; a normal reconnect alone is insufficient.
- [ ] **HW-4.2 — negative track admission.** Pre-load, preview, Cue, stopped,
  non-playing, unmapped, and nonmatching identity do not trigger. Ambiguous
  identity requires a bounded injected protocol case.
- [ ] **HW-4.3 — any-deck exact playback.** Play an exact mapped track on every
  intended deck; any Master display state is irrelevant to admission. Require one `DJ_TRACK_ACTIVE`, one
  admitted owner, and exactly one mapped Timeline start.
- [ ] **HW-4.4 — concurrent owner fencing.** A second playing deck cannot steal
  the admitted owner. Terminally release it, then prove a later mapped deck or
  session can acquire ownership.
- [ ] **HW-4.5 — Stage 1 LoopHalf.** F14 sends local MIDI and Rekordbox reports
  repeated absolute measured `DJ_LOOP_STATE` down through `1/64`. Record MIDI
  monitor and payload timestamps. A true no-response prediction is separate and
  must never replace a stale/invalid/contradictory measurement.
- [ ] **HW-4.6 — Stage 1 release policy.** Current controlled v1.1.9 requires
  `releaseMacro.enabled=true`, exact sequence `filter-then-fade-then-stop`, and
  `releaseFade.enabled=true`. On the accepted F13 edge, prove that owner-channel
  HPF CC16 starts `64 -> 127` over `1000 ms` in `50 ms` updates and exactly one
  correlated `DJ_RELEASE` is routed to Syndocal before local MIDI completion.
  After HPF completes, prove the independent ChannelFader CC17 leg
  `127 -> 0` over `1000 ms` in `50 ms` updates, exactly one Cue/Stop Note37,
  then HPF CC16 reset `127 -> 64` and fader CC17 reset `0 -> 127`. Syndocal
  Release delivery is independent of every local MIDI leg; a local HPF, fade,
  Stop, or reset failure remains visible and must not suppress that Release.
- [ ] **HW-4.7 — Release result.** F13 produces local Stop and one correlated
  Release. Only accepted/duplicate ACK succeeds. Rejection, timeout, disconnect,
  and send failure remain visible and fail closed; withheld/rejected ACK needs a
  bounded protocol/fault harness.
- [ ] **HW-4.8 — Stage 2.** Require authoritative `running`, exact
  timeline/play-session/pedal-owner/release correlation, and authoritative
  `loopActive:true`. F13 sends exactly one absolute
  `DJ_TIMELINE_LOOP_SET { active:false }`, releasing either the authored
  `人生オーバー` C-melody A-B loop or the post-Follow destination-first-measure
  runtime hold. The strict `transitionHoldActive` boolean remains required state,
  but is diagnostic for the latter and is not the F13 admission gate. A completed
  Follow rebase is required only to inherit authority for the post-Follow target;
  it is not required for the ordinary C-melody loop. F14 toggles the authoritative
  absolute loop; F15 alone sends `DJ_TIMELINE_BEAT_JUMP { bars:4 }`. `-4` and
  every Stage 2 Rekordbox MIDI action are rejected. Stale, abort, fault, or
  mismatched receipts remain fail-closed.
- [ ] **HW-4.9 — disconnect/reconnect.** Stage 1 local controls continue while
  disconnected. Reconnect requires fresh State Sync. Stage 2 stays fail-closed
  until a fresh authoritative snapshot exists.
- [ ] **HW-4.10 — dedupe/replay.** Exact same-session replay is duplicate with no
  second action; same ID with changed shape rejects. This requires a protocol
  harness because the normal UI cannot choose event IDs.
- [ ] **HW-4.11 — restart/next-show reuse.** Restart FOH and DJ Agent; prove the
  Credential Manager secret and exact NLM tuple rehydrate without redisplaying
  the token, then prove a new generation and fresh State Sync.
- [ ] **HW-4.12 — shared show LAN.** Run Art-Net/sACN concurrently and capture it
  on a separate physical receiver/computer. Same-host self-send is insufficient.

## 6. Abort and rollback

Abort the row without promotion on any unknown/stale/ambiguous identity,
unexpected fallback, duplicate physical action, missing ACK, lost Timeline
authority, secret exposure, wrong linker/artifact, or uncertain process target.
Preserve logs and screenshots before changing state. Do not rotate again merely
to hide a failed session; record the failed generation and cause first.

If the new build cannot reach the authenticated baseline, stop acceptance,
record the exact failure, and restore only a previously identified artifact by
an explicit operator decision. Do not call a restored older artifact alpha.17,
and do not mark any HW-4 row passed from the rollback.

## 7. Final manifest

```text
KDMX branch/HEAD/upstream:
Syndocal version/size/SHA256/PID:
DJ peer branch/HEAD/upstream/version:
Rekordbox / pedal / MIDI versions:
FOH NIC GUID/adapter GUID/IP:
DJ NIC/IP / switch path:
Credential generation / session (no token):
Rows passed: __ / 12
Rows still open and exact missing subchecks:
Art-Net/sACN receiver evidence:
ASIO/display/show-project evidence references:
Operator/date/time zone:
```

Commit the redacted manifest and update the acceptance/handoff documents only
after the named evidence is reviewed. A partially executed matrix remains
partially open; never round it up to completion.
