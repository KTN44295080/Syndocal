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
at execution time. The current checkpoint is alpha.26 source-only: native
alpha.26 build, artifact identity, launch, and UI acceptance remain
unverified. Never substitute an older source hash or an alpha.24-or-earlier
process as the current alpha.26 artifact identity.

### 1.1 Historical alpha.20 / Follow / Stage 2 execution gate — 2026-08-27

KDMX `1.2.0-alpha.20` was the source/native checkpoint at exact commit
`03b70cd14a285a41c63cfd1d9b3bd89c025eec16`. The immutable Follow /
Stage 2 source authority remains alpha.19 commit
`41faefc054a3c37cef81cfd2e69b4e3f3df5ab4f` and is carried forward without a
wire or policy change.
The v1.1.9 runtime implementation provenance is the external peer commit
`b03d66a87b8d9dcdedfbd9b5c395bda7df7e0eec`; it is historical provenance, not
the current peer checkout identity. Follow hold,
Stage 2 F13 loop-off, reconnect fail-closed behavior, focused/full suites, and
independent source review are complete. The local alpha.20 native build was
launched as exactly one responsive maximized window, but target-DJ-PC identity,
configuration, token, NIC, and physical observations are still required before
any row is accepted. HW-4 remains **0/12** until those observations are recorded;
source or local native acceptance alone closes no hardware row.

### 1.2 Historical alpha.21 Setup I/O operator checkpoint — 2026-08-28

The historical I/O/native source checkpoint was committed and pushed at
`536742db968b242164349c34dd6940fe3ced8e92`
(`feat(setup): streamline the I/O operator workbench`), with product/file
version `1.2.0-alpha.21`. Setup I/O presents five selectors (DMX, MIDI, OSC,
Web Remote, DJ Link) and one full-width workbench body as the single scroll
owner. Its selector row uses `role=tablist`/`role=tab`, current-selection
semantics, and an exactly labelled `tabpanel`; the active workbench retains only
a compact useful label/state instead of redundant selected-workbench summary
text. Service and quick actions remain separate sibling
controls with independent hit targets. Dedicated browser contracts prove real
pointer hit-testing (including the fifth DJ tab),
keyboard selection (Enter/Space, ArrowLeft/Right, ArrowUp/Down, Home/End),
exact tab/tabpanel relation, strict overflow (`scrollHeight > clientHeight`),
and no competing scrollports among DMX/MIDI/OSC/Web/DJ descendants. The
remote-scroll contract opens Web
disclosures and scrolls the bottom `remote-standby` summary/control, requiring
visibility and a center hit-test. Web/DJ five-way and bottom-scroll proof is
browser-contract evidence only, not native visual proof.

Focused Setup I/O and remote-disclosure-scroll contracts passed at
`1920x1080`, `1920x1032`, `2048x1152`, `1366x768`, and `1280x720`.
Localization passed `3577/3577 (100%)` with `0` unprotected labels; frontend
typecheck/Vite build (`294` modules), JS syntax checks, and `git diff --check`
passed without first-party build warnings. Independent Terra xHigh final review
is GO with P0/P1/P2 all `0`.

The native gate used `vcvars64.bat -vcvars_ver=14.44`, pinned
`C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`
in `CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER`, and verified that exact path
first in `where.exe link.exe`. `pnpm --dir app tauri build --no-bundle`
produced `target/release/syndocal.exe`, Product/FileVersion
`1.2.0-alpha.21`, `58,778,112` bytes, SHA-256
`F73F1AD8F9E8229B8ACB713AE0C10C36E199C878D869A2E98B554190FC908FA3`.
Exactly one responsive maximized exact-checkout Syndocal window was observed
(PID `41912`). Native visual proof is limited to DMX/MIDI/OSC selector and
workbench paths; browser-only Web/DJ proof must not be promoted to native.

The separate token-free show structural preflight is read-only/authored-only
and is committed and pushed at exact
`b35d3ba351b29caaedfb7d0c6f4e29fc84580e83`. Tool/helper `node --check` and
`node qa/tests/show-structural-preflight.mjs` passed; the valid fixture exits
`0`, while legacy, missing, usage, and unknown inputs exit `2`. Independent
Terra xHigh final review is GO with P0/P1/P2 all `0`. It consumes no real token,
launches no show process, changes no runtime/project/hardware state, and cannot
advance HW-4.
It excludes production `.sdc`/media, target-DJ-PC checkout/config/token/NIC,
LAN/HELLO/ACK, Rekordbox, pedal/MIDI, reconnect/restart, three-display, and
physical-output proof. HW-4 remains exactly `0/12`; real production
`.sdc`/media, target-DJ-PC, LAN, and reconnect evidence remain unverified.
Cleanup is Plan-blocked after the clean upstream-equal `1d8fc4f` rerun:
`Mode=Plan`, `Outcome=Blocked`,
`Blocker=WriterOwnershipTopologyUnverifiable` (writer PID `61616` has missing
positive parent PID `49864`), `Candidates=[]`, `PlannedLogicalBytes=0`,
`ReclaimedLogicalBytes=0`; no deletion was performed.
The exact 12-path hardlink remediation was content-preserving with no content
diff.

### 1.3 Historical alpha.25 source/native operator checkpoint — 2026-08-28

The historical alpha.25 KDMX native artifact was built from the exact source
checkpoint `566a7101b0d5c9307e8d0efa5ccf499aba3eb404`. The final documentation
and evidence checkpoint carrying this binding is `c77b68f0d30075e7aed7de3d4273cc479c8f5abc`.
Product/FileVersion is `1.2.0-alpha.25`; the artifact is
`target/release/syndocal.exe`, `59,807,744` bytes, SHA-256
`D42B0A1B245197A76E3D3B5CC45DEEB1C6C33EDE6261C7DA94C34147141239C1`.

Exactly one responsive maximized exact-checkout Syndocal window was observed
(PID `83252`). Native visual proof covers the I/O DMX workbench and the
Lighting/Profile path only. Native Web Remote and DJ Link selector switching
remain unaccepted; their browser contracts are not native proof. The show
project/mapping, target-DJ-PC token and NIC, Rekordbox, pedal/MIDI, reconnect,
three-display, physical output, and all HW-4 observations remain open. HW-4 is
still exactly **0/12**.

### 1.4 Current alpha.26 source-only operator checkpoint — 2026-08-28

Product metadata is synchronized to `1.2.0-alpha.26` for the current App
structural split. Alpha.26 native build, artifact identity, launch, and native
UI acceptance are unbuilt/unverified; the historical alpha.25 artifact and
three-display runner must not be relabelled. The running alpha.25 DJ session is
preserved as an operational baseline only. Deck 1 showed `More One Night ×
動く、動く (Agate Trance&Makina bootleg)`, empty Artist, present Content ID,
position revision `149152`, BPM `140`, and `isPlaying=true`; DJ Agent sent
`DJ_TRACK_ACTIVE` event `22bfdf30-94e0-4264-9519-0ae4fba6bd2c`, delivery rejected,
FOH `owner— / playing No / track—`. The temporary track had no Timeline
mapping, so the result was fail-closed. Computer Use reproduced the Timeline
Start dropdown remount bug: selecting Timeline 1 immediately returned to the
placeholder; a separate Luna fix is in progress. This is not HW-4 end-to-end;
HW-4 remains exactly **0/12**.

Latest DJ-PC screenshot evidence is
`C:\TEMP\codex-clipboard-142f581b-eef4-41e9-80fa-a0a79b8c749e.png`, `218629`
bytes, SHA-256
`A217E511BD3905DE81148202E2746686CF320665BDAF5931ECEC0C2A9064F653`,
LastWriteTime `2026-08-28 07:10:32 JST`.

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
$ExpectedPeerHead = 'c22acaa265cbbc4936ab3af5b092b59d5d543f63'
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
`1.1.9`, exactly `c22acaa265cbbc4936ab3af5b092b59d5d543f63` for both `HEAD` and
`@{upstream}`, with strict adapter `syndocal-envelope-v3`. The v1.1.9 runtime
implementation ancestor is `b03d66a87b8d9dcdedfbd9b5c395bda7df7e0eec`; retain it
as provenance only and never accept it as the current checkout identity. The
current full peer suite passed `465` total / `463` pass / `0` fail / `2`
intentional skips. The commit is pushed, clean, and independently reviewed GO.
This is source/full-suite evidence only and closes no HW-4 row.
Branch name alone is insufficient. Do not use an installer, the blocked v1.1.3
release, or a historical v1.1.5/v1.1.6/v1.1.7/v1.1.8 configuration as current
acceptance evidence.

The preceding alpha.20 KDMX source/native checkpoint was `1.2.0-alpha.20` at branch
`codex/syndocal-v1.2`, exact commit
`03b70cd14a285a41c63cfd1d9b3bd89c025eec16`. The DJ
authority it carries is alpha.19 source commit
`41faefc054a3c37cef81cfd2e69b4e3f3df5ab4f`. At that alpha.19 checkpoint, the
exact MSVC 14.44 locked single-thread full workspace gate passed `2622` / `0`
failed / `15` intentional hardware-media ignores with zero first-party warnings,
and a clean detached non-OneDrive worktree passed
`pnpm --dir app run check:release` after staging the seven pinned runtime DLLs.
Those are historical alpha.19 results, not an alpha.20 release pass.

For historical alpha.20 context, frontend invokes were `422`, localization was `3582/3582`, and the
five-view Stage/Setup-I/O, stage labels, stage-live segments, TypeScript,
frontend production, and zero-warning gates passed. This active checkout's
tracked runtime-inventory hard-link alias still fails `check:release` before
metadata validation by design and was not deleted or replaced. The historical
alpha.20 `pnpm --dir app tauri build --no-bundle` artifact was Product/FileVersion
`1.2.0-alpha.20`, SHA-256
`E8100D160158034A63901EA1BF775EC06A48EFF4D97CAF454D0378C0D6988D7D`; it was
launched as exactly one responsive maximized window. It must not be substituted
for the historical alpha.25 artifact. Immediately before HW-4 execution, recheck
the historical alpha.25 identity recorded in section 1.3; every alpha.24-or-earlier artifact
or PID is historical.
The source UI now places Web Remote in the same
connection disclosure stack as DJ Link/Endpoints without shrinking controls;
the standard and dedicated Setup I/O browser contracts pass all five viewports,
including `1280x720`. An independent review found that rejected DMX network-route
buttons still changed the protocol draft before reporting `no state changed`;
the corrected candidate-only path now leaves the full draft and all `128` route
signatures unchanged, invokes neither retired output command, and independently
re-reviewed GO with P0/P1/P2 all zero. The earlier native visual capture belongs
only to the historical alpha.18 artifact; its direct native refresh-button click
also remains unconfirmed and neither observation accepts alpha.20. The
historical alpha.21 native Setup-I/O visual proof is recorded in section 1.2 and
the main pause handoff; historical alpha.25 native Web/DJ card switching remains
unaccepted because UI
Automation could not act on the cached element.
`app/dist` was freshly rebuilt (old stale
marker `0`); the ignored peer `dist` remained stale but was outside that source
acceptance checkpoint. These observations do not promote native or HW-4
acceptance.

### 2.1 Migrate an older external show config

An existing v1.1.8-or-earlier JSON is not a v1.1.9 config. Strict readiness
rejects it when the top-level `version` is not `1.1.9` or when the exact
production selector is absent: `trackActivity.ownerSelection.mode` must be
`titleContains`, `trackActivity.ownerSelection.titleNeedle` must be
`人生オーバー`, and `trackActivity.ownerSelection.deck1MetadataWaitMs` must be
`1400` (the bounded Deck 1 fallback wait). Do not edit, rename, or copy
the old JSON into the new target. The initializer must refuse an existing
target and never overwrite it; stop and resolve that conflict explicitly.

If the old file is the only available config, set `$OldConfigPath` to its
actual path (for example `C:\SyndocalShow\dj-agent-v1.1.8.json`) and run this
in the same target-checkout PowerShell. The token is held only in memory and is
never printed:

```powershell
$OldConfigPath = 'C:\SyndocalShow\dj-agent-v1.1.8.json' # use the actual older file
$NewConfigPath = 'C:\SyndocalShow\dj-agent-v1.1.9.json'
if (-not (Test-Path -LiteralPath $OldConfigPath -PathType Leaf)) {
  throw 'The older external show config was not found.'
}
if (Test-Path -LiteralPath $NewConfigPath -PathType Leaf) {
  throw 'The v1.1.9 target already exists; initializer refusal is fail-closed and no overwrite is allowed.'
}
$OldConfig = [System.IO.File]::ReadAllText(
  $OldConfigPath,
  [System.Text.Encoding]::UTF8
) | ConvertFrom-Json
$TokenInMemory = [string]$OldConfig.syndocal.token
if ($TokenInMemory.Length -lt 32 -or $TokenInMemory.Length -gt 256 -or
    $TokenInMemory -match '\s' -or
    $TokenInMemory -eq '<SYNDOCAL_ONE_TIME_TOKEN>') {
  throw 'The older config token is not reusable; rotate it in Syndocal and enter it without printing it.'
}
.\start-all.bat --init-config
if (-not (Test-Path -LiteralPath $NewConfigPath -PathType Leaf)) {
  throw 'The initializer did not create the v1.1.9 target.'
}
$NewConfig = [System.IO.File]::ReadAllText(
  $NewConfigPath,
  [System.Text.Encoding]::UTF8
) | ConvertFrom-Json
if ($NewConfig.version -ne '1.1.9' -or
    $NewConfig.trackActivity.ownerSelection.mode -ne 'titleContains' -or
    $NewConfig.trackActivity.ownerSelection.titleNeedle -ne '人生オーバー' -or
    $NewConfig.trackActivity.ownerSelection.deck1MetadataWaitMs -ne 1400) {
  throw 'Initializer output failed the exact v1.1.9 production owner-selection check.'
}
$NewConfig.syndocal.token = $TokenInMemory
$Utf8NoBom = [System.Text.UTF8Encoding]::new($false)
[System.IO.File]::WriteAllText(
  $NewConfigPath,
  ($NewConfig | ConvertTo-Json -Depth 12),
  $Utf8NoBom
)
$env:DJ_AGENT_CONFIG_PATH = $NewConfigPath
$TokenInMemory = $null
$OldConfig = $null
$NewConfig = $null
```

Continue with the forbidden-override check below, then run `--preflight-only`
and the real no-argument launch. Do not print the token while checking the
result; record only configured/length-valid status and non-secret generation or
session identifiers.

If the initializer has already created the exact v1.1.9 target but a
Windows PowerShell 5.1 default-encoding read corrupted only the Unicode owner
selector, do not rerun the initializer and do not reuse a stale in-memory
template object. Repair that existing target explicitly as follows. This path
requires one regular non-link file, reads and writes UTF-8 explicitly, preserves
the configured token without printing it, and constructs `人生オーバー` from
Unicode code points so neither the console code page nor a Japanese checkout
path can alter the selector:

```powershell
$NewConfigPath = 'C:\SyndocalShow\dj-agent-v1.1.9.json'
$NewConfigItem = Get-Item -LiteralPath $NewConfigPath -Force
if (-not $NewConfigItem.PSIsContainer -and
    -not ($NewConfigItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint)) {
  $NewConfig = [System.IO.File]::ReadAllText(
    $NewConfigPath,
    [System.Text.Encoding]::UTF8
  ) | ConvertFrom-Json
} else {
  throw 'The existing v1.1.9 target is not a regular non-link file.'
}
if ($NewConfig.version -ne '1.1.9') {
  throw 'This recovery applies only to an existing v1.1.9 target.'
}
$TrackActivityKeys = @($NewConfig.trackActivity.PSObject.Properties.Name)
$OwnerSelectionKeys = @($NewConfig.trackActivity.ownerSelection.PSObject.Properties.Name)
if ($TrackActivityKeys.Count -ne 1 -or
    $TrackActivityKeys[0] -cne 'ownerSelection' -or
    $OwnerSelectionKeys.Count -ne 3 -or
    @($OwnerSelectionKeys | Where-Object {
      $_ -notin @('mode', 'titleNeedle', 'deck1MetadataWaitMs')
    }).Count -ne 0) {
  throw 'The existing target differs beyond the Unicode owner selector; do not repair it in place.'
}
$TokenInMemory = [string]$NewConfig.syndocal.token
if ($TokenInMemory.Length -lt 32 -or $TokenInMemory.Length -gt 256 -or
    $TokenInMemory -match '\s' -or
    $TokenInMemory -eq '<SYNDOCAL_ONE_TIME_TOKEN>') {
  throw 'The existing target token is not reusable; rotate it in Syndocal and enter it without printing it.'
}
$TitleNeedle = -join (
  0x4EBA, 0x751F, 0x30AA, 0x30FC, 0x30D0, 0x30FC |
    ForEach-Object { [char]$_ }
)
$NewConfig.trackActivity.ownerSelection = [pscustomobject]@{
  mode = 'titleContains'
  titleNeedle = $TitleNeedle
  deck1MetadataWaitMs = 1400
}
$Utf8NoBom = [System.Text.UTF8Encoding]::new($false)
[System.IO.File]::WriteAllText(
  $NewConfigPath,
  ($NewConfig | ConvertTo-Json -Depth 32),
  $Utf8NoBom
)
$VerifiedConfig = [System.IO.File]::ReadAllText(
  $NewConfigPath,
  [System.Text.Encoding]::UTF8
) | ConvertFrom-Json
if ($VerifiedConfig.version -ne '1.1.9' -or
    $VerifiedConfig.trackActivity.ownerSelection.mode -ne 'titleContains' -or
    $VerifiedConfig.trackActivity.ownerSelection.titleNeedle -cne $TitleNeedle -or
    $VerifiedConfig.trackActivity.ownerSelection.deck1MetadataWaitMs -ne 1400 -or
    [string]$VerifiedConfig.syndocal.token -cne $TokenInMemory) {
  throw 'The repaired v1.1.9 target failed exact local verification.'
}
$env:DJ_AGENT_CONFIG_PATH = $NewConfigPath
$TokenInMemory = $null
$NewConfig = $null
$VerifiedConfig = $null
$TitleNeedle = $null
$TrackActivityKeys = $null
$OwnerSelectionKeys = $null
```

Continue with the forbidden-override check and strict launcher preflight below.
The launcher remains the final contract validator; a locally repaired file that
still differs anywhere else fails closed and must not start the show process.

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

### 3.1 2026-08-28 recovered connection (pre-matrix evidence only)

After creating the exact external v1.1.9 configuration, the only remaining
non-secret differential observed before the final retry was the forbidden
process override `SYNDOCAL_TOKEN`. The override was removed from the same
PowerShell before the next controlled no-argument launch. The separate CLI
preflight success line was not preserved, so this record does not claim an
isolated before/after causal proof. At `2026-08-28T07:07:22+09:00`, the exact
alpha.25 `target/release/syndocal.exe` PID `83252` was responsive and owned both
the `192.168.50.1:9100` listener and an established session from
`192.168.50.2:63235`. The unique maximized I/O > DJ Link workbench reported
`接続済み`, generation `1`, and a live heartbeat near `758 ms`.

This is accepted as connection-recovery evidence only. The initial authoritative
Timeline snapshot has not yet been re-recorded, and none of the MIDI, pedal,
track-admission, fault, reconnect, or shared-LAN subchecks below has run.
Accordingly the ordered matrix remains exactly **0/12**.

The operator then captured the DJ-PC extension with Deck 1 in `PLAY`, title
`More One Night × 動く、動く (Agate Trance&Makina bootleg)`, realtime/track
BPM `140.00`, and Artist displayed as `-`; the screenshot did not expose a
Content ID. The fresh FOH card simultaneously remained
`所有デッキ — / 再生中 No / トラック —` with the heartbeat live. This does
not prove a fallback bug: strict v3 requires either a nonempty Content ID or
complete title+Artist identity before the 1400 ms fallback timer may arm, and
also requires a fresh position sample/revision. Treat this as an
identity-incomplete diagnostic observation only. Preserve playback or switch
to a Deck 1 track with visible Artist, then capture the non-secret per-deck
identity/freshness fields before classifying any fallback or negative-admission
subcheck.

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
  non-playing, unmapped, ambiguous mapping, and a nonmatching identity outside
  the explicitly authored Deck 1 fallback do not trigger. Deck 2 must never
  enter that fallback. Ambiguous mapping requires a bounded injected protocol
  case.
- [ ] **HW-4.3 — any-deck mapped playback and explicit fallback arbitration.**
  Play an exact mapped track on every intended deck; any Master display state
  is irrelevant to admission. Then prove the production selector separately:
  zero title positives may select only fresh playing Deck 1 after the 1400 ms
  metadata wait; multiple positives prefer fresh playing Deck 1 and otherwise
  the lowest valid positive deck. Require one `DJ_TRACK_ACTIVE`, one
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
