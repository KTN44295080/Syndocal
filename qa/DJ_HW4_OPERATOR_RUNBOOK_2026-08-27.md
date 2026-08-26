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
at execution time. Never substitute an older alpha.16 source hash or the
previous alpha.15 process as the artifact identity.

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
$ExpectedDocsHead = '789f7724a699324cd87171ef835b69486bcd4e70'
$RequiredRuntimeCheckpoint = 'ee2f6c3148f36dfd63e0b70e2ab372247dbb8572'
$branch = (git branch --show-current).Trim()
$dirty = @(git status --porcelain=v1)
$head = (git rev-parse HEAD).Trim()
$upstream = (git rev-parse '@{upstream}').Trim()
$version = (node -p "require('./package.json').version").Trim()
git merge-base --is-ancestor $RequiredRuntimeCheckpoint $head
$runtimePresent = ($LASTEXITCODE -eq 0)
if ($branch -ne $ExpectedBranch -or $dirty.Count -ne 0 -or
    $head -ne $ExpectedDocsHead -or $upstream -ne $ExpectedDocsHead -or
    $version -ne '1.1.6' -or -not $runtimePresent) {
  throw 'DJ-PC checkout failed exact controlled-source identity validation.'
}
[pscustomobject]@{
  Root=$PeerRoot; Branch=$branch; Clean=($dirty.Count -eq 0)
  Head=$head; Upstream=$upstream; Version=$version
  RuntimeCheckpointPresent=$runtimePresent
}
```

The required identities are the clean upstream-equal docs tip
`789f7724a699324cd87171ef835b69486bcd4e70` containing runtime checkpoint
`ee2f6c3148f36dfd63e0b70e2ab372247dbb8572`, source version `1.1.6`, and strict
adapter `syndocal-envelope-v3`. Branch name alone is insufficient. Do not use
an installer, the blocked v1.1.3 release, or a historical v1.1.5 configuration
as current acceptance evidence.

If and only if the v1.1.6 external configuration is absent:

```powershell
.\start-all.bat --init-config
```

Edit only `C:\SyndocalShow\dj-agent-v1.1.6.json`. Replace the one-time token
placeholder and verify the exact `CustomMIDI1` name and port. Then, in the same
PowerShell:

```powershell
$env:DJ_AGENT_CONFIG_PATH = 'C:\SyndocalShow\dj-agent-v1.1.6.json'
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
  intended deck, including a non-Master deck. Require one `DJ_TRACK_ACTIVE`, one
  admitted owner, and exactly one mapped Timeline start.
- [ ] **HW-4.4 — concurrent owner fencing.** A second playing deck cannot steal
  the admitted owner. Terminally release it, then prove a later mapped deck or
  session can acquire ownership.
- [ ] **HW-4.5 — Stage 1 LoopHalf.** F14 sends local MIDI and Rekordbox reports
  repeated absolute measured `DJ_LOOP_STATE` down through `1/64`. Record MIDI
  monitor and payload timestamps. A true no-response prediction is separate and
  must never replace a stale/invalid/contradictory measurement.
- [ ] **HW-4.6 — Stage 1 release policy.** Current controlled v1.1.6 accepts
  only `releaseMacro.enabled=false`; prove the direct local Stop plus independent
  Syndocal Release path. Do not enable or select filter-then-fade during HW-4:
  that mode requires a separately implemented, reviewed, and deployed source
  change. If the show still requires that macro, stop and leave this row open.
- [ ] **HW-4.7 — Release result.** F13 produces local Stop and one correlated
  Release. Only accepted/duplicate ACK succeeds. Rejection, timeout, disconnect,
  and send failure remain visible and fail closed; withheld/rejected ACK needs a
  bounded protocol/fault harness.
- [ ] **HW-4.8 — Stage 2.** With authoritative `running`, F13/F15 perform
  Timeline `-4/+4`, F14 applies the absolute loop, and no MIDI is emitted.
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
an explicit operator decision. Do not call the restored alpha.15 artifact
alpha.16, and do not mark any HW-4 row passed from the rollback.

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
