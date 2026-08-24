# Focused deterministic tests for qa/harnesses/observe-syndocal-window-geometry.ps1.
#
# Modes
#   1. Standalone self-test (primary, zero external dependencies):
#        pwsh qa/harnesses/tests/observe-syndocal-window-geometry.Tests.ps1
#      Prints one PASS/FAIL line per check plus a SUMMARY line; exit 0 when all
#      checks pass, exit 1 otherwise.
#   2. Pester 3.x compatible layer: Invoke-Pester over this directory runs the
#      identical checks as Describe/It blocks. The It bodies assert through
#      Assert-CheckFocusedPassed, which uses 'Should -Be' on Pester >=4 and
#      the only syntax Pester 3.4.0's Should parses ('Should Be') on 3.x.
#
# The harness is DOT-SOURCED (never executed): its invocation guard defines
# functions only, so every Win32/native boundary below is exercised through
# explicit seam overrides. Each orchestration run saves all seam functions and
# restores them in finally; checks configure their complete stub world up front
# and are order-independent.
#
# Checks are pure: no process is started or stopped, no UI is operated, no
# window is touched, no network is used, and no file is written. The only
# real-world reads are the harness source text (static check) and, in every
# check that drives Invoke-SyndocalGeometryObservation end to end
# (DPI-restoration plus the full-stub-world window-contract checks),
# Test-Path/Get-Item/Get-FileHash on this checkout's existing
# target\release\syndocal.exe plus read-only user32 DPI context queries on
# this thread.
#
# Coverage map (required focused cases):
#   exact identity accepted .................... exact identity accepted
#   unreadable candidate fails closed .......... image-path throw + empty variants,
#                                                CreateFileW-open + identity-info variants
#   duplicate exact candidates rejected ........ duplicate exact candidates rejected
#   lexical alias same identity accepted ....... lexical alias with same identity accepted
#   lexical-equal different-identity rejected .. lexically equal different identity rejected
#   DPI restoration after forced exception ..... PMv2 thread DPI context restored after forced exception
#   stdout-only/no-file/no-encoding contract ... static stdout-only contract
#   duplicate eligible main windows rejected ... two titled main windows rejected
#   wrong-title/no eligible main rejected ...... zero eligible main windows rejected
#   HWND vanishes mid-observation rejected ..... vanished main HWND rejected
#   monitor info failure rejected .............. GetMonitorInfo failure rejected
#   DPI query failure/zero rejected ............ GetDpiForWindow failure/zero rejected
#   hung main window rejected .................. unresponsive main window rejected
#   wrong-owner main window rejected ........... foreign-owner main window rejected
#   success shape guarded ...................... full stub world emits schema_version 2

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$script:HarnessPath = Join-Path `
  (Split-Path -Parent (Split-Path -Parent $PSCommandPath)) `
  "observe-syndocal-window-geometry.ps1"

$script:RunningUnderPester = $false
foreach ($stackFrame in (Get-PSCallStack)) {
  if ($stackFrame.Command -match "(?i)pester") {
    $script:RunningUnderPester = $true
    break
  }
}

. $script:HarnessPath

$script:SeamFunctionNames = @(
  "Get-SyndocalCandidateProcessIds",
  "Get-NativeProcessImagePath",
  "Open-StableIdentityHandle",
  "Get-FinalPathFromHandle",
  "Get-FileIdentityFromHandle",
  "Get-VisibleTopLevelWindowsForPid",
  "Test-NativeWindowAlive",
  "Get-WindowOwnerProcessId",
  "Test-WindowNotHung",
  "Get-ObservationWindowRectangles",
  "Get-ObservationMonitorInfo",
  "Get-EffectiveDpiForWindow",
  "Get-ObservationWindowStateFlags"
)

function Save-Seams {
  $saved = @{}
  foreach ($seamName in $script:SeamFunctionNames) {
    $saved[$seamName] = (Get-Item -LiteralPath "function:$seamName").ScriptBlock
  }
  return $saved
}

function Restore-Seams {
  param([Parameter(Mandatory = $true)][hashtable]$Saved)

  foreach ($seamName in $Saved.Keys) {
    Set-Item -LiteralPath "function:$seamName" -Value $Saved[$seamName]
  }
}

function Set-Seam {
  param(
    [Parameter(Mandatory = $true)][string]$Name,
    [Parameter(Mandatory = $true)][scriptblock]$Body
  )

  Set-Item -LiteralPath "function:$Name" -Value $Body
}

function New-StubIdentity {
  param(
    [uint32]$VolumeSerialNumber = [uint32]2882400001,
    [uint64]$FileIndex = [uint64]424242
  )

  [pscustomobject]@{
    VolumeSerialNumber = $VolumeSerialNumber
    FileIndex = $FileIndex
  }
}

function New-SynthTargetFingerprint {
  # Synthetic fingerprint matching the default stub world; RequestedPath is
  # display-only and never touches the real filesystem.
  param([uint64]$FileIndex = [uint64]424242)

  [pscustomobject]@{
    RequestedPath = "C:\synth\baseline\syndocal.exe"
    FinalPath = "C:\synth\checkout\target\release\syndocal.exe"
    Identity = New-StubIdentity -VolumeSerialNumber ([uint32]2882400001) -FileIndex $FileIndex
  }
}

function New-CheckOutcome {
  param([bool]$Passed, [string]$Detail = "")

  [pscustomobject]@{ Passed = $Passed; Detail = $Detail }
}

function Assert-CheckFocusedPassed {
  # Dual-Pester assertion. Pester 3.4.0's Should accepts ONLY the undashed
  # operator form (its Parse-ShouldArgs turns the first raw argument into
  # "PesterBe"; '-Be' yields the error ''-Be' is not a valid Should
  # operator'), while Pester 5+ removed the legacy form and accepts only
  # 'Should -Be'. Branch on the loaded Pester major so each host executes the
  # exact syntax it supports, warning-free.
  param([Parameter(Mandatory = $true)]$Check)

  $loadedPester = @(Get-Module Pester | Select-Object -First 1)[0]
  if ($null -ne $loadedPester -and $loadedPester.Version.Major -ge 4) {
    $Check.Passed | Should -Be $true
  } else {
    $Check.Passed | Should Be $true
  }
}

function New-StubRect {
  param(
    [int]$Left = 0,
    [int]$Top = 0,
    [int]$Right = 800,
    [int]$Bottom = 600
  )

  $rect = [SyndocalGeometryObserverNative+RECT]::new()
  $rect.Left = $Left
  $rect.Top = $Top
  $rect.Right = $Right
  $rect.Bottom = $Bottom
  $rect
}

function New-StubPoint {
  param([int]$X = 8, [int]$Y = 30)

  $point = [SyndocalGeometryObserverNative+POINT]::new()
  $point.X = $X
  $point.Y = $Y
  $point
}

function New-StubWindowListEntry {
  param([Parameter(Mandatory = $true)][long]$HandleValue, [string]$Title)

  [pscustomobject]@{
    Handle = [IntPtr]$HandleValue
    Title = $Title
  }
}

function Set-FullObservationStubWorld {
  # Installs a complete deterministic stub world in which
  # Invoke-SyndocalGeometryObservation reaches the report stage and succeeds:
  # one exact-identity candidate process and exactly one titled "Syndocal"
  # main window with valid geometry probes. Individual checks then override
  # exactly one seam to force a specific failure deterministically. The only
  # real reads left are Test-Path/Get-Item/Get-FileHash on this checkout's
  # release exe inside the harness.
  param(
    [uint32]$TargetPid = [uint32]47001,
    [object[]]$Windows = $null
  )

  $script:FullWorldPid = $TargetPid
  if ($null -eq $Windows) {
    $script:FullWorldWindows =
      @(New-StubWindowListEntry -HandleValue 8101 -Title "Syndocal")
  } else {
    $script:FullWorldWindows = @($Windows)
  }

  Set-Seam "Get-SyndocalCandidateProcessIds" { [uint32[]]@($script:FullWorldPid) }
  Set-Seam "Get-NativeProcessImagePath" {
    param($ProcessId)
    "C:\synth\checkout\target\release\syndocal.exe"
  }
  Set-Seam "Open-StableIdentityHandle" { param($Path) [IntPtr]3101 }
  Set-Seam "Get-FinalPathFromHandle" {
    param($Handle)
    "\\?\C:\synth\checkout\target\release\syndocal.exe"
  }
  Set-Seam "Get-FileIdentityFromHandle" { param($Handle) New-StubIdentity }
  Set-Seam "Get-VisibleTopLevelWindowsForPid" {
    param($OwnerPid)
    @($script:FullWorldWindows | Sort-Object -Property { $_.Handle.ToInt64() })
  }
  Set-Seam "Test-NativeWindowAlive" { param($Handle) $true }
  Set-Seam "Get-WindowOwnerProcessId" {
    param($Handle)
    [uint32]$script:FullWorldPid
  }
  Set-Seam "Test-WindowNotHung" { param($Handle) $true }
  Set-Seam "Get-ObservationWindowRectangles" {
    param($Handle)
    [pscustomobject]@{
      Outer = New-StubRect -Left 100 -Top 80 -Right 1124 -Bottom 848
      Client = New-StubRect -Left 0 -Top 0 -Right 1008 -Bottom 730
      ClientOrigin = New-StubPoint -X 108 -Y 110
    }
  }
  Set-Seam "Get-ObservationMonitorInfo" {
    param($Handle)
    [pscustomobject]@{
      Size = 104
      DeviceName = "\\.\DISPLAY1"
      Monitor = (New-StubRect -Left 0 -Top 0 -Right 1920 -Bottom 1080)
      Work = (New-StubRect -Left 0 -Top 40 -Right 1920 -Bottom 1080)
    }
  }
  Set-Seam "Get-EffectiveDpiForWindow" { param($Handle) [uint32]96 }
  Set-Seam "Get-ObservationWindowStateFlags" {
    param($Handle)
    [pscustomobject]@{
      Visible = $true
      Responding = $true
      Minimized = $false
      Maximized = $true
    }
  }
}

function Invoke-ExpectThrow {
  param(
    [Parameter(Mandatory = $true)][scriptblock]$Action,
    [Parameter(Mandatory = $true)][string[]]$MustContain
  )

  try {
    & $Action | Out-Null
    return [pscustomobject]@{ Threw = $false; Message = ""; MissingToken = $null }
  } catch {
    $thrownMessage = $_.Exception.Message
    foreach ($needle in $MustContain) {
      if (-not $thrownMessage.Contains($needle)) {
        return [pscustomobject]@{ Threw = $true; Message = $thrownMessage; MissingToken = $needle }
      }
    }
    return [pscustomobject]@{ Threw = $true; Message = $thrownMessage; MissingToken = $null }
  }
}

function Invoke-CheckExactIdentityAccepted {
  Set-Seam "Get-NativeProcessImagePath" {
    param($ProcessId)
    "C:\synth\checkout\target\release\syndocal.exe"
  }
  Set-Seam "Open-StableIdentityHandle" { param($Path) [IntPtr]1001 }
  Set-Seam "Get-FinalPathFromHandle" {
    param($Handle)
    "\\?\C:\synth\checkout\target\release\syndocal.exe"
  }
  Set-Seam "Get-FileIdentityFromHandle" { param($Handle) New-StubIdentity }

  try {
    $selectedPidValue = Select-SyndocalTargetProcessId `
      -CandidateProcessIds ([uint32[]]@(41001)) `
      -TargetFingerprint (New-SynthTargetFingerprint)
    if ($selectedPidValue -eq 41001) {
      return New-CheckOutcome -Passed $true -Detail "PID 41001 selected"
    }
    return New-CheckOutcome -Passed $false -Detail "expected PID 41001, got '$selectedPidValue'"
  } catch {
    return New-CheckOutcome -Passed $false -Detail "unexpected throw: $($_.Exception.Message)"
  }
}

function Invoke-CheckUnreadableImagePathFailsClosed {
  Set-Seam "Get-NativeProcessImagePath" {
    param($ProcessId)
    throw ("SIMULATED-UNREADABLE OpenProcess denied for PID {0}" -f $ProcessId)
  }

  $outcome = Invoke-ExpectThrow `
    -Action { Select-SyndocalTargetProcessId -CandidateProcessIds ([uint32[]]@(42001)) -TargetFingerprint (New-SynthTargetFingerprint) } `
    -MustContain @("SIMULATED-UNREADABLE", "42001")
  if (-not $outcome.Threw) {
    return New-CheckOutcome -Passed $false -Detail "selection did NOT fail closed on unreadable candidate"
  }
  if ($outcome.MissingToken) {
    return New-CheckOutcome -Passed $false -Detail "message missing '$($outcome.MissingToken)': $($outcome.Message)"
  }
  return New-CheckOutcome -Passed $true -Detail "propagated: $($outcome.Message)"
}

function Invoke-CheckIncompleteImagePathFailsClosed {
  Set-Seam "Get-NativeProcessImagePath" { param($ProcessId) "" }

  $outcome = Invoke-ExpectThrow `
    -Action { Select-SyndocalTargetProcessId -CandidateProcessIds ([uint32[]]@(42002)) -TargetFingerprint (New-SynthTargetFingerprint) } `
    -MustContain @("incomplete", "42002")
  if (-not $outcome.Threw) {
    return New-CheckOutcome -Passed $false -Detail "empty image path did NOT fail closed"
  }
  if ($outcome.MissingToken) {
    return New-CheckOutcome -Passed $false -Detail "message missing '$($outcome.MissingToken)': $($outcome.Message)"
  }
  return New-CheckOutcome -Passed $true -Detail "rejected empty/incomplete image path"
}

function Invoke-CheckDuplicateExactCandidatesRejected {
  Set-Seam "Get-NativeProcessImagePath" {
    param($ProcessId)
    "C:\synth\checkout\target\release\syndocal.exe"
  }
  Set-Seam "Open-StableIdentityHandle" { param($Path) [IntPtr]1003 }
  Set-Seam "Get-FinalPathFromHandle" {
    param($Handle)
    "\\?\C:\synth\checkout\target\release\syndocal.exe"
  }
  Set-Seam "Get-FileIdentityFromHandle" { param($Handle) New-StubIdentity }

  $outcome = Invoke-ExpectThrow `
    -Action { Select-SyndocalTargetProcessId -CandidateProcessIds ([uint32[]]@(43001, 43002)) -TargetFingerprint (New-SynthTargetFingerprint) } `
    -MustContain @("2 exact identity matches", "43001", "43002")
  if (-not $outcome.Threw) {
    return New-CheckOutcome -Passed $false -Detail "duplicate exact matches were NOT rejected"
  }
  if ($outcome.MissingToken) {
    return New-CheckOutcome -Passed $false -Detail "message missing '$($outcome.MissingToken)': $($outcome.Message)"
  }
  return New-CheckOutcome -Passed $true -Detail $outcome.Message
}

function Invoke-CheckLexicalAliasSameIdentityAccepted {
  # Candidate reports an 8.3/casing-mangled ALIAS path that is lexically very
  # different, but its handle resolves to the SAME final path and SAME
  # volume/file identity as the target -> MUST be accepted.
  Set-Seam "Get-NativeProcessImagePath" {
    param($ProcessId)
    "C:\SYNTH~1\CHECKOUT\tarGET\Release\SYNDOCAL.Exe"
  }
  Set-Seam "Open-StableIdentityHandle" { param($Path) [IntPtr]1004 }
  Set-Seam "Get-FinalPathFromHandle" {
    param($Handle)
    "\\?\C:\synth\checkout\target\release\syndocal.exe"
  }
  Set-Seam "Get-FileIdentityFromHandle" {
    param($Handle)
    New-StubIdentity -VolumeSerialNumber ([uint32]2882400001) -FileIndex ([uint64]424242)
  }

  try {
    $selectedPidValue = Select-SyndocalTargetProcessId `
      -CandidateProcessIds ([uint32[]]@(44004)) `
      -TargetFingerprint (New-SynthTargetFingerprint)
    if ($selectedPidValue -eq 44004) {
      return New-CheckOutcome -Passed $true -Detail "alias resolved to same native identity; PID 44004 accepted"
    }
    return New-CheckOutcome -Passed $false -Detail "expected PID 44004, got '$selectedPidValue'"
  } catch {
    return New-CheckOutcome -Passed $false -Detail "lexical alias wrongly rejected: $($_.Exception.Message)"
  }
}

function Invoke-CheckLexicalEqualDifferentIdentityRejected {
  # Candidate reports a lexically IDENTICAL path to the target, but the stable
  # volume/file identity differs (rebuilt exe generation) -> MUST be rejected;
  # raw lexical equality alone is never proof.
  Set-Seam "Get-NativeProcessImagePath" {
    param($ProcessId)
    "C:\synth\checkout\target\release\syndocal.exe"
  }
  Set-Seam "Open-StableIdentityHandle" { param($Path) [IntPtr]1005 }
  Set-Seam "Get-FinalPathFromHandle" {
    param($Handle)
    "\\?\C:\synth\checkout\target\release\syndocal.exe"
  }
  Set-Seam "Get-FileIdentityFromHandle" {
    param($Handle)
    New-StubIdentity -VolumeSerialNumber ([uint32]2882400001) -FileIndex ([uint64]999999)
  }

  $outcome = Invoke-ExpectThrow `
    -Action { Select-SyndocalTargetProcessId -CandidateProcessIds ([uint32[]]@(44005)) -TargetFingerprint (New-SynthTargetFingerprint) } `
    -MustContain @("0 exact identity matches", "44005")
  if (-not $outcome.Threw) {
    return New-CheckOutcome -Passed $false -Detail "lexically equal but different-identity candidate was NOT rejected"
  }
  if ($outcome.MissingToken) {
    return New-CheckOutcome -Passed $false -Detail "message missing '$($outcome.MissingToken)': $($outcome.Message)"
  }
  return New-CheckOutcome -Passed $true -Detail $outcome.Message
}

function Invoke-CheckZeroCandidatesRejected {
  $outcome = Invoke-ExpectThrow `
    -Action { Select-SyndocalTargetProcessId -CandidateProcessIds ([uint32[]]@()) -TargetFingerprint (New-SynthTargetFingerprint) } `
    -MustContain @("found 0 syndocal-named processes")
  if (-not $outcome.Threw) {
    return New-CheckOutcome -Passed $false -Detail "zero candidates were NOT rejected"
  }
  if ($outcome.MissingToken) {
    return New-CheckOutcome -Passed $false -Detail "message missing '$($outcome.MissingToken)': $($outcome.Message)"
  }
  return New-CheckOutcome -Passed $true -Detail $outcome.Message
}

function Invoke-CheckUnreadableIdentityOpenFailsClosed {
  Set-Seam "Get-NativeProcessImagePath" {
    param($ProcessId)
    "C:\synth\checkout\target\release\syndocal.exe"
  }
  Set-Seam "Open-StableIdentityHandle" {
    param($Path)
    throw ("SIMULATED-CREATEFILE-FAILURE for '{0}'" -f $Path)
  }

  $outcome = Invoke-ExpectThrow `
    -Action { Select-SyndocalTargetProcessId -CandidateProcessIds ([uint32[]]@(45001)) -TargetFingerprint (New-SynthTargetFingerprint) } `
    -MustContain @("SIMULATED-CREATEFILE-FAILURE", "45001")
  if (-not $outcome.Threw) {
    return New-CheckOutcome -Passed $false -Detail "unreadable identity handle did NOT fail closed"
  }
  if ($outcome.MissingToken) {
    return New-CheckOutcome -Passed $false -Detail "message missing '$($outcome.MissingToken)': $($outcome.Message)"
  }
  return New-CheckOutcome -Passed $true -Detail "CreateFileW seam failure propagated fail-closed"
}

function Invoke-CheckUnreadableIdentityQueryFailsClosed {
  Set-Seam "Get-NativeProcessImagePath" {
    param($ProcessId)
    "C:\synth\checkout\target\release\syndocal.exe"
  }
  Set-Seam "Open-StableIdentityHandle" { param($Path) [IntPtr]1007 }
  Set-Seam "Get-FinalPathFromHandle" {
    param($Handle)
    "\\?\C:\synth\checkout\target\release\syndocal.exe"
  }
  Set-Seam "Get-FileIdentityFromHandle" {
    param($Handle)
    throw "SIMULATED-GFIBH-FAILURE GetFileInformationByHandle returned FALSE"
  }

  $outcome = Invoke-ExpectThrow `
    -Action { Select-SyndocalTargetProcessId -CandidateProcessIds ([uint32[]]@(45002)) -TargetFingerprint (New-SynthTargetFingerprint) } `
    -MustContain @("SIMULATED-GFIBH-FAILURE", "45002")
  if (-not $outcome.Threw) {
    return New-CheckOutcome -Passed $false -Detail "unreadable file identity did NOT fail closed"
  }
  if ($outcome.MissingToken) {
    return New-CheckOutcome -Passed $false -Detail "message missing '$($outcome.MissingToken)': $($outcome.Message)"
  }
  return New-CheckOutcome -Passed $true -Detail "identity query failure propagated fail-closed"
}

function Invoke-CheckDpiRestoredAfterForcedException {
  # Fully synthetic end-to-end wiring up to the window seam; the forced throw
  # happens AFTER SetThreadDpiAwarenessContext succeeded, so restoration in
  # finally is the only way this check can pass. Real reads: Test-Path,
  # Get-Item and Get-FileHash on this checkout's existing release exe, plus
  # user32 DPI context queries on THIS thread only.
  Set-Seam "Get-SyndocalCandidateProcessIds" { [uint32[]]@(44001) }
  Set-Seam "Get-NativeProcessImagePath" {
    param($ProcessId)
    "C:\synth\checkout\target\release\syndocal.exe"
  }
  Set-Seam "Open-StableIdentityHandle" { param($Path) [IntPtr]2002 }
  Set-Seam "Get-FinalPathFromHandle" {
    param($Handle)
    "\\?\C:\synth\checkout\target\release\syndocal.exe"
  }
  Set-Seam "Get-FileIdentityFromHandle" {
    param($Handle)
    New-StubIdentity -VolumeSerialNumber ([uint32]2882400001) -FileIndex ([uint64]424242)
  }
  Set-Seam "Get-VisibleTopLevelWindowsForPid" {
    param($OwnerPid)
    throw "FORCED-FOR-TEST simulated EnumWindows detonation inside the DPI-protected region"
  }

  $contextBefore = [SyndocalGeometryObserverNative]::GetThreadDpiAwarenessContext()
  $threw = $false
  $forcedSeen = $false
  $thrownMessage = ""
  try {
    Invoke-SyndocalGeometryObservation | Out-Null
  } catch {
    $threw = $true
    $thrownMessage = $_.Exception.Message
    $forcedSeen = $thrownMessage.Contains("FORCED-FOR-TEST")
  }
  $contextAfter = [SyndocalGeometryObserverNative]::GetThreadDpiAwarenessContext()

  if (-not $threw) {
    return New-CheckOutcome -Passed $false -Detail "observation unexpectedly succeeded despite forced window-seam failure"
  }
  if (-not $forcedSeen) {
    return New-CheckOutcome -Passed $false -Detail "forced exception did not reach the DPI-protected region: $thrownMessage"
  }
  if ($contextBefore -ne $contextAfter) {
    return New-CheckOutcome -Passed $false -Detail (
      "DPI context NOT restored: before={0} after={1}" -f $contextBefore, $contextAfter)
  }
  return New-CheckOutcome -Passed $true -Detail (
    "thread DPI context restored in finally (value {0}) after forced exception" -f $contextAfter)
}

function Invoke-CheckDuplicateMainWindowsRejected {
  # Two visible top-level windows of the proven PID both carry the exact main
  # title -> ambiguous; must fail closed before any geometry is reported.
  Set-FullObservationStubWorld -Windows @(
    (New-StubWindowListEntry -HandleValue 8201 -Title "Syndocal"),
    (New-StubWindowListEntry -HandleValue 8202 -Title "Syndocal")
  )
  $outcome = Invoke-ExpectThrow `
    -Action { Invoke-SyndocalGeometryObservation | Out-Null } `
    -MustContain @("expected exactly one 'Syndocal' main window", "found 2", "8201", "8202", "47001")
  if (-not $outcome.Threw) {
    return New-CheckOutcome -Passed $false -Detail "duplicate eligible main windows were NOT rejected"
  }
  if ($outcome.MissingToken) {
    return New-CheckOutcome -Passed $false -Detail "message missing '$($outcome.MissingToken)': $($outcome.Message)"
  }
  return New-CheckOutcome -Passed $true -Detail $outcome.Message
}

function Invoke-CheckNoEligibleMainTitleRejected {
  # Scenario A: windows exist but none carries the exact main title.
  Set-FullObservationStubWorld -Windows @(
    (New-StubWindowListEntry -HandleValue 8301 -Title "Daslight 4"),
    (New-StubWindowListEntry -HandleValue 8302 -Title "")
  )
  $outcomeA = Invoke-ExpectThrow `
    -Action { Invoke-SyndocalGeometryObservation | Out-Null } `
    -MustContain @("expected exactly one 'Syndocal' main window", "found 0", "Daslight 4", "47001")
  if (-not $outcomeA.Threw) {
    return New-CheckOutcome -Passed $false -Detail "wrong-title world did NOT fail closed"
  }
  if ($outcomeA.MissingToken) {
    return New-CheckOutcome -Passed $false -Detail "scenario A missing '$($outcomeA.MissingToken)': $($outcomeA.Message)"
  }

  # Scenario B: no visible top-level windows at all.
  Set-FullObservationStubWorld -Windows @()
  $outcomeB = Invoke-ExpectThrow `
    -Action { Invoke-SyndocalGeometryObservation | Out-Null } `
    -MustContain @("expected exactly one 'Syndocal' main window", "found 0", "<none>")
  if (-not $outcomeB.Threw) {
    return New-CheckOutcome -Passed $false -Detail "zero-window world did NOT fail closed"
  }
  if ($outcomeB.MissingToken) {
    return New-CheckOutcome -Passed $false -Detail "scenario B missing '$($outcomeB.MissingToken)': $($outcomeB.Message)"
  }
  return New-CheckOutcome -Passed $true -Detail "no eligible main title fails closed (with and without windows)"
}

function Invoke-CheckMainWindowVanishRejected {
  # The selected main HWND stops existing mid-observation -> fail closed.
  Set-FullObservationStubWorld
  Set-Seam "Test-NativeWindowAlive" { param($Handle) $false }
  $outcome = Invoke-ExpectThrow `
    -Action { Invoke-SyndocalGeometryObservation | Out-Null } `
    -MustContain @("vanished during observation", "8101")
  if (-not $outcome.Threw) {
    return New-CheckOutcome -Passed $false -Detail "vanished main HWND did NOT fail closed"
  }
  if ($outcome.MissingToken) {
    return New-CheckOutcome -Passed $false -Detail "message missing '$($outcome.MissingToken)': $($outcome.Message)"
  }
  return New-CheckOutcome -Passed $true -Detail $outcome.Message
}

function Invoke-CheckMonitorInfoFailureRejected {
  Set-FullObservationStubWorld
  Set-Seam "Get-ObservationMonitorInfo" {
    param($Handle)
    throw ("SIMULATED-MONITORINFO-FAILURE for HWND {0}" -f $Handle.ToInt64())
  }
  $outcome = Invoke-ExpectThrow `
    -Action { Invoke-SyndocalGeometryObservation | Out-Null } `
    -MustContain @("SIMULATED-MONITORINFO-FAILURE", "8101")
  if (-not $outcome.Threw) {
    return New-CheckOutcome -Passed $false -Detail "GetMonitorInfo failure did NOT fail closed"
  }
  if ($outcome.MissingToken) {
    return New-CheckOutcome -Passed $false -Detail "message missing '$($outcome.MissingToken)': $($outcome.Message)"
  }
  return New-CheckOutcome -Passed $true -Detail "monitor info failure propagated fail-closed"
}

function Invoke-CheckDpiQueryFailureOrZeroRejected {
  # Scenario A: DPI query throws -> propagation is fail closed.
  Set-FullObservationStubWorld
  Set-Seam "Get-EffectiveDpiForWindow" {
    param($Handle)
    throw "SIMULATED-DPIQUERY-FAILURE GetDpiForWindow exploded"
  }
  $outcomeA = Invoke-ExpectThrow `
    -Action { Invoke-SyndocalGeometryObservation | Out-Null } `
    -MustContain @("SIMULATED-DPIQUERY-FAILURE")
  if (-not $outcomeA.Threw) {
    return New-CheckOutcome -Passed $false -Detail "throwing DPI query did NOT fail closed"
  }
  if ($outcomeA.MissingToken) {
    return New-CheckOutcome -Passed $false -Detail "scenario A missing '$($outcomeA.MissingToken)': $($outcomeA.Message)"
  }

  # Scenario B: DPI query returns 0 -> unprovable scale factor rejected.
  Set-Seam "Get-EffectiveDpiForWindow" { param($Handle) [uint32]0 }
  $outcomeB = Invoke-ExpectThrow `
    -Action { Invoke-SyndocalGeometryObservation | Out-Null } `
    -MustContain @("GetDpiForWindow returned 0", "unprovable", "8101")
  if (-not $outcomeB.Threw) {
    return New-CheckOutcome -Passed $false -Detail "zero DPI result did NOT fail closed"
  }
  if ($outcomeB.MissingToken) {
    return New-CheckOutcome -Passed $false -Detail "scenario B missing '$($outcomeB.MissingToken)': $($outcomeB.Message)"
  }
  return New-CheckOutcome -Passed $true -Detail "DPI query failure and zero both fail closed"
}

function Invoke-CheckHungMainWindowRejected {
  # The exactly-one titled main window exists but IsHungAppWindow says hung ->
  # observation must fail closed instead of emitting success JSON.
  Set-FullObservationStubWorld
  Set-Seam "Test-WindowNotHung" { param($Handle) $false }
  $outcome = Invoke-ExpectThrow `
    -Action { Invoke-SyndocalGeometryObservation | Out-Null } `
    -MustContain @("not responding", "IsHungAppWindow reported hung", "8101")
  if (-not $outcome.Threw) {
    return New-CheckOutcome -Passed $false -Detail "hung main window did NOT fail closed"
  }
  if ($outcome.MissingToken) {
    return New-CheckOutcome -Passed $false -Detail "message missing '$($outcome.MissingToken)': $($outcome.Message)"
  }
  return New-CheckOutcome -Passed $true -Detail $outcome.Message
}

function Invoke-CheckWrongOwnerMainWindowRejected {
  # The titled window is owned by a DIFFERENT pid than the exact target ->
  # fail closed even though the title matched.
  Set-FullObservationStubWorld
  Set-Seam "Get-WindowOwnerProcessId" { param($Handle) [uint32]65432 }
  $outcome = Invoke-ExpectThrow `
    -Action { Invoke-SyndocalGeometryObservation | Out-Null } `
    -MustContain @("owned by PID 65432", "not the proven target PID 47001", "8101")
  if (-not $outcome.Threw) {
    return New-CheckOutcome -Passed $false -Detail "wrong-owner main window did NOT fail closed"
  }
  if ($outcome.MissingToken) {
    return New-CheckOutcome -Passed $false -Detail "message missing '$($outcome.MissingToken)': $($outcome.Message)"
  }
  return New-CheckOutcome -Passed $true -Detail $outcome.Message
}

function Invoke-CheckFullObservationSuccessShape {
  # With every seam in its happy state the observation must emit ONE JSON
  # document with schema_version 2 and the guarded main-window fields intact.
  Set-FullObservationStubWorld
  try {
    $jsonText = Invoke-SyndocalGeometryObservation
  } catch {
    return New-CheckOutcome -Passed $false -Detail "unexpected throw: $($_.Exception.Message)"
  }
  $parsed = $jsonText | ConvertFrom-Json

  $assertions = @(
    @{ Name = "schema_version"; Ok = ($parsed.schema_version -eq 2) },
    @{ Name = "tool"; Ok = ($parsed.tool -eq "observe-syndocal-window-geometry") },
    @{ Name = "target pid"; Ok = ([uint64]$parsed.target_executable.pid -eq [uint64]47001) },
    @{ Name = "matched_process_count"; Ok = ([int]$parsed.process_match.matched_process_count -eq 1) },
    @{ Name = "main_window_count"; Ok = ([int]$parsed.main_window_count -eq 1) },
    @{ Name = "main hwnd"; Ok = ([uint64]$parsed.main_window.hwnd_decimal -eq [uint64]8101) },
    @{ Name = "main title"; Ok = ($parsed.main_window.title -eq "Syndocal") },
    @{ Name = "main responding"; Ok = ($parsed.main_window.responding -eq $true) },
    @{ Name = "main effective_dpi"; Ok = ([uint64]$parsed.main_window.effective_dpi -eq [uint64]96) },
    @{ Name = "main scale_factor"; Ok = ([double]$parsed.main_window.scale_factor -eq 1.0) },
    @{ Name = "window count"; Ok = (@($parsed.windows).Count -eq 1) },
    @{ Name = "target_process_modified"; Ok = ($parsed.coordinate_contract.target_process_modified -eq $false) },
    @{ Name = "state_changing_ui_api_calls"; Ok = ([int]$parsed.coordinate_contract.state_changing_ui_api_calls -eq 0) }
  )
  foreach ($assertion in $assertions) {
    if (-not $assertion.Ok) {
      return New-CheckOutcome -Passed $false -Detail "success-shape assertion failed: $($assertion.Name)"
    }
  }
  return New-CheckOutcome -Passed $true -Detail "one schema_version 2 document with guarded main-window fields"
}

function Invoke-CheckStaticOutputContract {
  $harnessText = Get-Content -LiteralPath $script:HarnessPath -Raw

  foreach ($forbiddenToken in @(
    @{ Token = "OutputPath"; Why = "file-output surface must be removed entirely" },
    @{ Token = "[Console]::OutputEncoding"; Why = "console encoding mutation prohibited" },
    @{ Token = "Write-ObservationJsonAtomic"; Why = "atomic file writer must be removed" },
    @{ Token = "Write-Host"; Why = "stdout must stay a single JSON object" },
    @{ Token = "Out-File"; Why = "harness writes nothing" },
    @{ Token = "Set-Content"; Why = "harness writes nothing" },
    @{ Token = "Add-Content"; Why = "harness writes nothing" },
    @{ Token = "Start-Process"; Why = "observer never starts processes" },
    @{ Token = "Stop-Process"; Why = "observer never stops processes" }
  )) {
    if ($harnessText.Contains($forbiddenToken.Token)) {
      return New-CheckOutcome -Passed $false -Detail (
        "forbidden token '{0}' present ({1})" -f $forbiddenToken.Token, $forbiddenToken.Why)
    }
  }

  foreach ($requiredToken in @(
    "CreateFileW",
    "GetFinalPathNameByHandleW",
    "GetFileInformationByHandle",
    "QueryFullProcessImageNameW",
    "SetThreadDpiAwarenessContext",
    "IsWindow",
    "GetWindowThreadProcessId",
    "GetMonitorInfo",
    "GetDpiForWindow",
    "IsHungAppWindow",
    "Test-NativeWindowAlive",
    "Get-WindowOwnerProcessId",
    "Test-WindowNotHung",
    "vanished during observation",
    "not responding",
    "finally",
    "-Name `$script:SyndocalProcessName",
    "schema_version = 2"
  )) {
    if (-not $harnessText.Contains($requiredToken)) {
      return New-CheckOutcome -Passed $false -Detail "required token '$requiredToken' missing from harness"
    }
  }

  $parseErrors = $null
  $parsedAst = [System.Management.Automation.Language.Parser]::ParseFile(
    $script:HarnessPath, [ref]$null, [ref]$parseErrors)
  if (@($parseErrors).Count -gt 0) {
    $firstError = @($parseErrors)[0]
    return New-CheckOutcome -Passed $false -Detail (
      "parse error at {0}:{1}: {2}" -f $firstError.Extent.StartLineNumber, $firstError.Extent.StartColumnNumber, $firstError.Message)
  }

  $scriptParamBlock = $parsedAst.ParamBlock
  if ($null -eq $scriptParamBlock -or @($scriptParamBlock.Parameters).Count -ne 0) {
    return New-CheckOutcome -Passed $false -Detail (
      "expected a single empty script-level param block; found {0}" -f
      $(if ($null -eq $scriptParamBlock) { "no script-level param block" } else { "$(@($scriptParamBlock.Parameters).Count) parameter(s)" }))
  }

  return New-CheckOutcome -Passed $true -Detail "stdout-only contract holds; zero parse errors; empty param block"
}

function Invoke-AllFocusedChecks {
  $results = [System.Collections.Generic.List[object]]::new()
  $savedSeams = Save-Seams
  try {
    $checks = @(
      @{ Name = "exact identity accepted"; Run = { Invoke-CheckExactIdentityAccepted } },
      @{ Name = "unreadable candidate image-path query fails closed"; Run = { Invoke-CheckUnreadableImagePathFailsClosed } },
      @{ Name = "incomplete empty image path fails closed"; Run = { Invoke-CheckIncompleteImagePathFailsClosed } },
      @{ Name = "duplicate exact candidates rejected"; Run = { Invoke-CheckDuplicateExactCandidatesRejected } },
      @{ Name = "lexical alias with same native identity accepted"; Run = { Invoke-CheckLexicalAliasSameIdentityAccepted } },
      @{ Name = "lexically equal different identity rejected"; Run = { Invoke-CheckLexicalEqualDifferentIdentityRejected } },
      @{ Name = "zero candidates rejected"; Run = { Invoke-CheckZeroCandidatesRejected } },
      @{ Name = "unreadable CreateFileW open fails closed"; Run = { Invoke-CheckUnreadableIdentityOpenFailsClosed } },
      @{ Name = "unreadable file-identity query fails closed"; Run = { Invoke-CheckUnreadableIdentityQueryFailsClosed } },
      @{ Name = "PMv2 thread DPI context restored after forced exception"; Run = { Invoke-CheckDpiRestoredAfterForcedException } },
      @{ Name = "two eligible main windows rejected"; Run = { Invoke-CheckDuplicateMainWindowsRejected } },
      @{ Name = "wrong-title/no eligible main window rejected"; Run = { Invoke-CheckNoEligibleMainTitleRejected } },
      @{ Name = "main HWND vanishing mid-observation rejected"; Run = { Invoke-CheckMainWindowVanishRejected } },
      @{ Name = "GetMonitorInfo failure rejected"; Run = { Invoke-CheckMonitorInfoFailureRejected } },
      @{ Name = "GetDpiForWindow failure/zero rejected"; Run = { Invoke-CheckDpiQueryFailureOrZeroRejected } },
      @{ Name = "hung (unresponsive) main window rejected"; Run = { Invoke-CheckHungMainWindowRejected } },
      @{ Name = "wrong-owner main window rejected"; Run = { Invoke-CheckWrongOwnerMainWindowRejected } },
      @{ Name = "full stub world emits schema_version 2 success shape"; Run = { Invoke-CheckFullObservationSuccessShape } },
      @{ Name = "static stdout-only contract"; Run = { Invoke-CheckStaticOutputContract } }
    )
    foreach ($checkDefinition in $checks) {
      try {
        $checkOutcome = & $checkDefinition.Run
        $passedValue = [bool]$checkOutcome.Passed
        $detailValue = [string]$checkOutcome.Detail
      } catch {
        $passedValue = $false
        $detailValue = "UNEXPECTED: $($_.Exception.Message)"
      }
      $results.Add([pscustomobject]@{
        Name = $checkDefinition.Name
        Passed = $passedValue
        Detail = $detailValue
      })
    }
  } finally {
    Restore-Seams -Saved $savedSeams
  }
  return @($results)
}

if ($script:RunningUnderPester) {
  Describe "observe-syndocal-window-geometry focused checks" {
    $focusedResults = Invoke-AllFocusedChecks
    foreach ($resultItem in @($focusedResults)) {
      $capturedResult = $resultItem
      It $capturedResult.Name { Assert-CheckFocusedPassed -Check $capturedResult }
    }
  }
} elseif ($MyInvocation.InvocationName -ne ".") {
  # Standalone deterministic self-test run.
  $focusedResults = Invoke-AllFocusedChecks
  foreach ($resultItem in $focusedResults) {
    $statusText = "FAIL"
    if ($resultItem.Passed) { $statusText = "PASS" }
    $lineText = "{0} {1}" -f $statusText, $resultItem.Name
    if ($resultItem.Passed -and $resultItem.Detail) {
      $lineText = "{0} ({1})" -f $lineText, $resultItem.Detail
    } elseif (-not $resultItem.Passed -and $resultItem.Detail) {
      $lineText = "{0} -- {1}" -f $lineText, $resultItem.Detail
    }
    $lineText
  }
  $failedCount = @($focusedResults | Where-Object { -not $_.Passed }).Count
  "SUMMARY: $(@($focusedResults).Count) checks, $failedCount failed"
  if ($failedCount -eq 0) { exit 0 } else { exit 1 }
}
# Manually dot-sourcing outside Pester defines functions without side effects.
