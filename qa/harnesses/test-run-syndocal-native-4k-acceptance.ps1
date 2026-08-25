# Focused deterministic tests for qa/harnesses/run-syndocal-native-4k-acceptance.ps1.
#
# PRIMARY MODE (standalone, zero external dependencies):
#   pwsh       qa/harnesses/test-run-syndocal-native-4k-acceptance.ps1
#   powershell qa/harnesses/test-run-syndocal-native-4k-acceptance.ps1   (PS 5.1)
# Prints PASS/FAIL per check plus a SUMMARY line; exit 0 iff all pass.
# A Pester compatibility layer mirrors the checks when run under Pester.
#
# ISOLATION CONTRACT
#   The runner is DOT-SOURCED (its invocation guard defines functions only).
#   Every native/process/window boundary is an explicit SEAM override; these
#   tests NEVER enumerate or mutate a real window or process, never touch the
#   network, take no screenshots, and perform no process termination. All
#   real filesystem traffic stays inside this run's own temp sandbox
#   (%TEMP%\syndocal-accept-selftest-*), including a junction used only to
#   prove reparse rejection. Nothing under the repository's qa/artifacts tree
#   is created or modified.
#
# COVERAGE MAP (required proofs -> checks)
#   dry-run default ............................ dry_run_default_performs_zero_mutations,
#                                                dry_run_writes_complete_evidence_set,
#                                                dry_run_criteria_informational_only
#   exact identity rejection ................... apply_missing_field_* (x6),
#                                                malformed_sha256/git_head,
#                                                relative_path/wrong_leaf,
#                                                off-pin identifier/resolution/dpi,
#                                                observed sha/version/head mismatch
#   duplicate/missing monitor .................. zero_matching_monitors_*,
#                                                duplicate_monitor_tuple_*,
#                                                unavailable_description_fails_apply
#   wrong HWND/PID/hash/version ................ vanish_before_second_op,
#                                                ownership_change_before_second_op,
#                                                hash_flip_after_first_op,
#                                                version_flip_before_second_op,
#                                                unresponsive_before_second_op,
#                                                window_left_target_monitor_before_op
#   pre-change revalidation .................... every abort check above asserts
#                                                the mutation seam was NOT reached;
#                                                happy worlds assert per-operation
#                                                revalidation blocks in operation.json
#   traversal/reparse/owned-dir rejection ...... traversal_slugs_rejected,
#                                                reserved_device_names_rejected,
#                                                trailing_dot_slug_rejected,
#                                                existing_candidate_never_reused,
#                                                junction_in_ancestry_rejected,
#                                                owner_sid_decision_matrix
#   stable sample success/failure .............. already_compliant_world_* (success),
#                                                instability_resets_and_exhausts (failure),
#                                                delay_seam_proves_sample_separation
#   Apply authorization ........................ all authorization checks above +
#                                                interval/attempts floors
#   no process termination ..................... static_contract_forbidden_tokens_absent
#                                                (termination/capture/network/child-
#                                                window APIs absent from runner text)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$script:TestFilePath = $PSCommandPath
$script:RunnerPath = Join-Path (Split-Path -Parent $PSCommandPath) "run-syndocal-native-4k-acceptance.ps1"

$script:RunningUnderPester = $false
foreach ($stackFrame in (Get-PSCallStack)) {
  if ($stackFrame.Command -match "(?i)pester") {
    $script:RunningUnderPester = $true
    break
  }
}

. $script:RunnerPath

# ---------------------------------------------------------------------------
# Seam save / restore / override
# ---------------------------------------------------------------------------

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
  "Get-ObservationWindowStateFlags",
  "Get-ExecutableSha256",
  "Get-ExecutableProductVersion",
  "Resolve-GitHeadFromRepository",
  "Get-DisplayMonitorIdentities",
  "Get-MonitorHandleForWindow",
  "Invoke-ShowWindowState",
  "Invoke-SetWindowPlacementRect",
  "Invoke-SampleDelay"
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

# ---------------------------------------------------------------------------
# Check plumbing
# ---------------------------------------------------------------------------

function New-CheckOutcome {
  param([bool]$Passed, [string]$Detail = "")
  [pscustomobject]@{ Passed = $Passed; Detail = $Detail }
}

function Assert-CheckFocusedPassed {
  # Dual-Pester assertion (Pester 3.x legacy form vs Pester 4+ dashed form).
  param([Parameter(Mandatory = $true)]$Check)

  $loadedPester = @(Get-Module Pester | Select-Object -First 1)[0]
  if ($null -ne $loadedPester -and $loadedPester.Version.Major -ge 4) {
    $Check.Passed | Should -Be $true
  } else {
    $Check.Passed | Should Be $true
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

function Assert-ThrowContains {
  param(
    [Parameter(Mandatory = $true)][scriptblock]$Action,
    [Parameter(Mandatory = $true)][string[]]$MustContain,
    [Parameter(Mandatory = $true)][string]$FailDetail
  )

  $outcome = Invoke-ExpectThrow -Action $Action -MustContain $MustContain
  if (-not $outcome.Threw) {
    return New-CheckOutcome -Passed $false -Detail "$FailDetail : did NOT throw"
  }
  if ($outcome.MissingToken) {
    return New-CheckOutcome -Passed $false -Detail "$FailDetail : message missing '$($outcome.MissingToken)': $($outcome.Message)"
  }
  return New-CheckOutcome -Passed $true -Detail $outcome.Message
}

function Assert-ResultFailedAtStage {
  param(
    [Parameter(Mandatory = $true)]$Result,
    [Parameter(Mandatory = $true)][string]$ExpectedStage,
    [Parameter(Mandatory = $true)][string[]]$ErrorTokens,
    [Parameter(Mandatory = $true)][string]$FailDetail
  )

  if ([bool]$Result.Succeeded) {
    return New-CheckOutcome -Passed $false -Detail "$FailDetail : run unexpectedly succeeded"
  }
  if ([string]$Result.Stage -ne $ExpectedStage) {
    return New-CheckOutcome -Passed $false -Detail "$FailDetail : stage '$($Result.Stage)' != '$ExpectedStage'; errors: $($Result.Errors -join ' | ')"
  }
  foreach ($token in $ErrorTokens) {
    $found = $false
    foreach ($errorMessage in @($Result.Errors)) {
      if ([string]$errorMessage -ne $null -and $errorMessage.Contains($token)) { $found = $true; break }
    }
    if (-not $found) {
      return New-CheckOutcome -Passed $false -Detail "$FailDetail : no error contains '$token'; errors: $($Result.Errors -join ' | ')"
    }
  }
  return New-CheckOutcome -Passed $true -Detail "failed closed at '$ExpectedStage' ($($Result.Errors.Count) error(s))"
}

# ---------------------------------------------------------------------------
# Temp sandbox (all real filesystem traffic lives here)
# ---------------------------------------------------------------------------

$script:SandboxRoot = Join-Path ([IO.Path]::GetTempPath()) (
  "syndocal-accept-selftest-" + [Guid]::NewGuid().ToString("N"))
$script:SandboxExePath = ""

function New-TestSandbox {
  if (-not (Test-Path -LiteralPath $script:SandboxRoot)) {
    [void](New-Item -ItemType Directory -Path $script:SandboxRoot -Force)
  }
}

function New-FreshEvidenceRoot {
  $rootPath = Join-Path $script:SandboxRoot ("ev-" + [Guid]::NewGuid().ToString("N"))
  [void](New-Item -ItemType Directory -Path $rootPath -Force)
  return $rootPath
}

function New-SyntheticTargetExecutable {
  $dirPath = Join-Path $script:SandboxRoot "synth\checkout\target\release"
  [void](New-Item -ItemType Directory -Path $dirPath -Force)
  $exePath = Join-Path $dirPath "syndocal.exe"
  [IO.File]::WriteAllText(
    $exePath,
    "synthetic stand-in executable for deterministic acceptance-runner self-tests",
    (New-Object Text.UTF8Encoding($false)))
  $script:SandboxExePath = $exePath
  return $exePath
}

function Remove-TestSandbox {
  if (Test-Path -LiteralPath $script:SandboxRoot) {
    Remove-Item -LiteralPath $script:SandboxRoot -Recurse -Force
  }
}

# ---------------------------------------------------------------------------
# Stub world constants + state
# ---------------------------------------------------------------------------

$script:GoodSha256 = ("a7" * 32)
$script:BadSha256 = ("ff" * 32)
$script:GoodVersion = "1.2.3-selftest"
$script:BadVersion = "9.9.9-wrong"
$script:GoodHead = ("beef" * 10)
$script:BadHead = ("dead" * 10)
$script:StubPid = [uint32]47111
$script:ForeignPid = [uint32]99999
$script:StubHwndDecimal = [long]9101
$script:TargetMonitorHandle = [long]7001
$script:OtherMonitorHandle = [long]7002

$script:ShowWindowCalls = [System.Collections.Generic.List[string]]::new()
$script:SetWindowPosCalls = [System.Collections.Generic.List[string]]::new()
$script:DelayCalls = [System.Collections.Generic.List[int]]::new()

$script:ShaCallCount = 0
$script:ShaGoodUntil = [int]::MaxValue
$script:VersionCallCount = 0
$script:VersionGoodUntil = [int]::MaxValue
$script:OwnerCallCount = 0
$script:OwnerGoodUntil = [int]::MaxValue
$script:AliveCallCount = 0
$script:AliveGoodUntil = [int]::MaxValue
$script:HungCallCount = 0
$script:HungGoodUntil = [int]::MaxValue
$script:MonForWinCallCount = 0
$script:MonForWinSequence = @()
$script:StateFlagsCallCount = 0
$script:StateFlagsMode = "always_maximized"

function New-StubRect {
  param(
    [int]$Left = 0,
    [int]$Top = 0,
    [int]$Right = 3840,
    [int]$Bottom = 2160
  )

  $rect = [SyndocalAcceptanceRunnerNative+RECT]::new()
  $rect.Left = $Left
  $rect.Top = $Top
  $rect.Right = $Right
  $rect.Bottom = $Bottom
  $rect
}

function New-StubPoint {
  param([int]$X = 0, [int]$Y = 0)

  $point = [SyndocalAcceptanceRunnerNative+POINT]::new()
  $point.X = $X
  $point.Y = $Y
  $point
}

function New-StubIdentity {
  [pscustomobject]@{
    VolumeSerialNumber = [uint32]2882400001
    FileIndex = [uint64]424242
  }
}

function New-StubMonitorIdentity {
  param(
    [long]$HandleValue = $script:TargetMonitorHandle,
    [string]$Description = "MSI3DD2",
    [int]$EffectiveDpi = 144,
    [int]$Width = 3840,
    [int]$Height = 2160
  )

  [pscustomobject]@{
    handle_decimal = $HandleValue
    device_name = "\\.\DISPLAY4"
    description = $Description
    description_source = $(if ($Description -eq "") { "unavailable" } else { "stub" })
    effective_dpi = $EffectiveDpi
    physical_bounds = [pscustomobject]@{ left = 0; top = 0; right = $Width; bottom = $Height; width = $Width; height = $Height }
    work_area = [pscustomobject]@{ left = 0; top = 0; right = $Width; bottom = $Height; width = $Width; height = $Height }
  }
}

function Reset-StubCounters {
  $script:ShaCallCount = 0
  $script:ShaGoodUntil = [int]::MaxValue
  $script:VersionCallCount = 0
  $script:VersionGoodUntil = [int]::MaxValue
  $script:OwnerCallCount = 0
  $script:OwnerGoodUntil = [int]::MaxValue
  $script:AliveCallCount = 0
  $script:AliveGoodUntil = [int]::MaxValue
  $script:HungCallCount = 0
  $script:HungGoodUntil = [int]::MaxValue
  $script:MonForWinCallCount = 0
  $script:MonForWinSequence = @()
  $script:StateFlagsCallCount = 0
  $script:ShowWindowCalls.Clear()
  $script:SetWindowPosCalls.Clear()
  $script:DelayCalls.Clear()
}

function Set-CoreStubWorld {
  # Installs the complete deterministic happy world: one exact-identity
  # syndocal process owning exactly one titled "Syndocal" main window that is
  # alive, responsive, maximized, on the target monitor tuple, and already
  # compliant. Individual checks then flip exactly one knob.
  param(
    [string]$StateFlagsMode = "always_maximized",
    [long[]]$NearestMonitorSequence = @(),
    [object[]]$MonitorIdentities = @()
  )

  Reset-StubCounters
  $script:StateFlagsMode = $StateFlagsMode
  if ($NearestMonitorSequence.Count -gt 0) {
    $script:MonForWinSequence = @($NearestMonitorSequence)
  } else {
    $script:MonForWinSequence = @([long]$script:TargetMonitorHandle)
  }
  if ($MonitorIdentities.Count -gt 0) {
    $script:MonitorIdentitiesWorld = @($MonitorIdentities)
  } else {
    $script:MonitorIdentitiesWorld = @(New-StubMonitorIdentity)
  }

  # --- observer seams (process identity) -----------------------------------
  Set-Seam "Get-SyndocalCandidateProcessIds" { [uint32[]]@($script:StubPid) }
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

  # --- observer seams (window probes) ---------------------------------------
  Set-Seam "Get-VisibleTopLevelWindowsForPid" {
    param($OwnerPid)
    @([pscustomobject]@{ Handle = [IntPtr]::new($script:StubHwndDecimal); Title = "Syndocal" })
  }
  Set-Seam "Test-NativeWindowAlive" {
    param($Handle)
    $script:AliveCallCount++
    return ($script:AliveCallCount -le $script:AliveGoodUntil)
  }
  Set-Seam "Get-WindowOwnerProcessId" {
    param($Handle)
    $script:OwnerCallCount++
    if ($script:OwnerCallCount -le $script:OwnerGoodUntil) { return [uint32]$script:StubPid }
    return [uint32]$script:ForeignPid
  }
  Set-Seam "Test-WindowNotHung" {
    param($Handle)
    $script:HungCallCount++
    return ($script:HungCallCount -le $script:HungGoodUntil)
  }
  Set-Seam "Get-ObservationWindowRectangles" {
    param($Handle)
    [pscustomobject]@{
      Outer = New-StubRect -Left 0 -Top 0 -Right 3840 -Bottom 2160
      Client = New-StubRect -Left 0 -Top 0 -Right 3840 -Bottom 2088
      ClientOrigin = New-StubPoint -X 0 -Y 0
    }
  }
  Set-Seam "Get-ObservationMonitorInfo" {
    param($Handle)
    [pscustomobject]@{
      DeviceName = "\\.\DISPLAY4"
      Monitor = (New-StubRect)
      Work = (New-StubRect)
    }
  }
  Set-Seam "Get-EffectiveDpiForWindow" { param($Handle) [uint32]144 }
  Set-Seam "Get-ObservationWindowStateFlags" {
    param($Handle)
    $script:StateFlagsCallCount++
    $callIndex = $script:StateFlagsCallCount
    $minimized = $false
    $maximized = $false
    switch ($script:StateFlagsMode) {
      "always_maximized" { $maximized = $true }
      "small_then_maximized" {
        if ($callIndex -eq 1) { $maximized = $false } else { $maximized = $true }
      }
      "minimized_then_maximized" {
        if ($callIndex -eq 1) { $minimized = $true } else { $maximized = $true }
      }
      "always_minimized" { $minimized = $true }
      "unstable_every_third" {
        if (($callIndex % 3) -eq 0) { $maximized = $false } else { $maximized = $true }
      }
      default { throw "Unknown StateFlagsMode '$($script:StateFlagsMode)'" }
    }
    [pscustomobject]@{
      Visible = $true
      Responding = $true
      Minimized = $minimized
      Maximized = $maximized
    }
  }

  # --- runner seams (external facts) -----------------------------------------
  Set-Seam "Get-ExecutableSha256" {
    param($Path)
    $script:ShaCallCount++
    if ($script:ShaCallCount -le $script:ShaGoodUntil) { return $script:GoodSha256 }
    return $script:BadSha256
  }
  Set-Seam "Get-ExecutableProductVersion" {
    param($Path)
    $script:VersionCallCount++
    if ($script:VersionCallCount -le $script:VersionGoodUntil) { return $script:GoodVersion }
    return $script:BadVersion
  }
  Set-Seam "Resolve-GitHeadFromRepository" { param($CheckoutRootPath) return $script:GoodHead }
  Set-Seam "Get-DisplayMonitorIdentities" { return @($script:MonitorIdentitiesWorld) }
  Set-Seam "Get-MonitorHandleForWindow" {
    param($HwndDecimal)
    $script:MonForWinCallCount++
    $sequence = @($script:MonForWinSequence)
    $index = $script:MonForWinCallCount - 1
    if ($index -ge $sequence.Count) { $index = $sequence.Count - 1 }
    return [long]$sequence[$index]
  }

  # --- runner seams (mutation + delay spies) ----------------------------------
  Set-Seam "Invoke-ShowWindowState" {
    param($Handle, $StateCommand)
    $script:ShowWindowCalls.Add("show:$StateCommand")
    return $true
  }
  Set-Seam "Invoke-SetWindowPlacementRect" {
    param($Handle, $X, $Y, $Width, $Height)
    $script:SetWindowPosCalls.Add("setpos:$X,$Y,$Width,$Height")
    return $true
  }
  Set-Seam "Invoke-SampleDelay" {
    param($Milliseconds)
    $script:DelayCalls.Add([int]$Milliseconds)
    return [int]$Milliseconds
  }
}

function New-AcceptanceRunParameters {
  param([hashtable]$Overrides = @{})

  $parameters = @{
    Apply = $true
    ExpectedExecutablePath = $script:SandboxExePath
    ExpectedSha256 = $script:GoodSha256
    ExpectedProductVersion = $script:GoodVersion
    ExpectedGitHead = $script:GoodHead
    ExpectedMonitorIdentifier = "MSI3DD2"
    ExpectedMonitorPhysicalResolution = "3840x2160"
    ExpectedEffectiveDpi = 144
    EvidenceSlug = "selftest"
    EvidenceRootPath = (New-FreshEvidenceRoot)
    SampleIntervalMs = 200
    MaxSampleAttempts = 6
  }
  foreach ($overrideKey in $Overrides.Keys) {
    $parameters[$overrideKey] = $Overrides[$overrideKey]
  }
  return $parameters
}

function Invoke-StubbedAcceptanceRun {
  param([hashtable]$ParameterOverrides = @{})

  $runParameters = New-AcceptanceRunParameters -Overrides $ParameterOverrides
  return Invoke-AcceptanceRun @runParameters
}

function Read-EvidenceJson {
  param(
    [Parameter(Mandatory = $true)][string]$EvidenceDirectory,
    [Parameter(Mandatory = $true)][string]$FileName
  )

  Get-Content -LiteralPath (Join-Path $EvidenceDirectory $FileName) -Raw | ConvertFrom-Json
}

# ---------------------------------------------------------------------------
# Checks: Apply authorization
# ---------------------------------------------------------------------------

function Invoke-CheckApplyWithoutExpectationsFailsClosed {
  Set-CoreStubWorld
  $result = Invoke-StubbedAcceptanceRun -ParameterOverrides @{
    ExpectedExecutablePath = ""
    ExpectedSha256 = ""
    ExpectedProductVersion = ""
    ExpectedGitHead = ""
    ExpectedMonitorIdentifier = ""
    ExpectedMonitorPhysicalResolution = ""
    ExpectedEffectiveDpi = 0
  }
  $stageOutcome = Assert-ResultFailedAtStage -Result $result -ExpectedStage "authorization" `
    -ErrorTokens @("-Apply requires explicit expectations") -FailDetail "apply without expectations"
  if (-not $stageOutcome.Passed) { return $stageOutcome }
  if ($null -ne $result.EvidencePath) {
    return New-CheckOutcome -Passed $false -Detail "authorization failure must precede evidence creation"
  }
  if (@($script:ShowWindowCalls).Count -ne 0 -or @($script:SetWindowPosCalls).Count -ne 0) {
    return New-CheckOutcome -Passed $false -Detail "mutation seams were reached despite authorization failure"
  }
  return New-CheckOutcome -Passed $true -Detail "no expectations -> authorization fails closed before any seam"
}

function Invoke-CheckApplyEachMissingFieldRejected {
  Set-CoreStubWorld
  $fieldNames = @(
    "ExpectedExecutablePath", "ExpectedSha256", "ExpectedProductVersion",
    "ExpectedGitHead", "ExpectedMonitorIdentifier", "ExpectedMonitorPhysicalResolution"
  )
  foreach ($fieldName in $fieldNames) {
    $result = Invoke-StubbedAcceptanceRun -ParameterOverrides @{ $fieldName = "" }
    if ([bool]$result.Succeeded -or [string]$result.Stage -ne "authorization" -or
        -not (($result.Errors -join " ").Contains($fieldName))) {
      return New-CheckOutcome -Passed $false -Detail (
        "missing {0} was not rejected at authorization; stage='{1}' errors='{2}'" -f
        $fieldName, $result.Stage, ($result.Errors -join " | "))
    }
    if (-not (([string]$result.Errors -join " ").Contains("-Apply requires explicit expectations"))) {
      return New-CheckOutcome -Passed $false -Detail "missing-field rejection lacks the explicit contract message"
    }
  }
  $dpiResult = Invoke-StubbedAcceptanceRun -ParameterOverrides @{ ExpectedEffectiveDpi = 0 }
  if (-not (([string]$dpiResult.Errors -join " ").Contains("ExpectedEffectiveDpi")) -or [bool]$dpiResult.Succeeded) {
    return New-CheckOutcome -Passed $false -Detail "missing ExpectedEffectiveDpi was not rejected"
  }
  return New-CheckOutcome -Passed $true -Detail "all 7 expectation fields individually mandatory with -Apply"
}

function Invoke-CheckApplyMalformedFormatsRejected {
  Set-CoreStubWorld
  $cases = @(
    @{ Overrides = @{ ExpectedSha256 = ("a7" * 31) }; Token = "malformed"; Name = "sha256 too short" },
    @{ Overrides = @{ ExpectedGitHead = ("beef" * 9) }; Token = "malformed"; Name = "git head too short" },
    @{ Overrides = @{ ExpectedExecutablePath = "target\release\syndocal.exe" }; Token = "must be an absolute path"; Name = "relative path" },
    @{ Overrides = @{ ExpectedExecutablePath = (Join-Path $script:SandboxRoot "other.exe") }; Token = "leaf must be exactly 'syndocal.exe'"; Name = "wrong leaf name" }
  )
  foreach ($case in $cases) {
    $outcome = Assert-ResultFailedAtStage -Result (
      Invoke-StubbedAcceptanceRun -ParameterOverrides $case.Overrides) `
      -ExpectedStage "authorization" -ErrorTokens @($case.Token) -FailDetail $case.Name
    if (-not $outcome.Passed) { return $outcome }
  }
  return New-CheckOutcome -Passed $true -Detail "sha/head format, absolute path, and syndocal.exe leaf all enforced"
}

function Invoke-CheckApplyOffPinMonitorTupleRejected {
  Set-CoreStubWorld
  $cases = @(
    @{ Overrides = @{ ExpectedMonitorIdentifier = "MSI3DD3" }; Token = "pinned target panel 'MSI3DD2'" },
    @{ Overrides = @{ ExpectedMonitorPhysicalResolution = "3440x1440" }; Token = "pinned '3840x2160'" },
    @{ Overrides = @{ ExpectedEffectiveDpi = 120 }; Token = "does not equal the pinned 144" }
  )
  foreach ($case in $cases) {
    $outcome = Assert-ResultFailedAtStage -Result (
      Invoke-StubbedAcceptanceRun -ParameterOverrides $case.Overrides) `
      -ExpectedStage "authorization" -ErrorTokens @($case.Token) -FailDetail ($case.Token)
    if (-not $outcome.Passed) { return $outcome }
  }
  return New-CheckOutcome -Passed $true -Detail "off-pin monitor identifier/resolution/dpi all rejected"
}

function Invoke-CheckSamplingFloorsEnforced {
  Set-CoreStubWorld
  $intervalOutcome = Assert-ResultFailedAtStage -Result (
    Invoke-StubbedAcceptanceRun -ParameterOverrides @{ SampleIntervalMs = 199 }) `
    -ExpectedStage "authorization" -ErrorTokens @("below the pinned minimum") -FailDetail "interval floor"
  if (-not $intervalOutcome.Passed) { return $intervalOutcome }
  $attemptsOutcome = Assert-ResultFailedAtStage -Result (
    Invoke-StubbedAcceptanceRun -ParameterOverrides @{ MaxSampleAttempts = 2 }) `
    -ExpectedStage "authorization" -ErrorTokens @("below the required consecutive") -FailDetail "attempts floor"
  if (-not $attemptsOutcome.Passed) { return $attemptsOutcome }
  return New-CheckOutcome -Passed $true -Detail "SampleIntervalMs>=200 and MaxSampleAttempts>=3 enforced structurally"
}

# ---------------------------------------------------------------------------
# Checks: dry-run default
# ---------------------------------------------------------------------------

function Invoke-CheckDryRunDefaultPerformsZeroMutations {
  Set-CoreStubWorld -StateFlagsMode "always_maximized"
  $result = Invoke-StubbedAcceptanceRun -ParameterOverrides @{ Apply = $false }
  if (-not [bool]$result.Succeeded) {
    return New-CheckOutcome -Passed $false -Detail "dry-run failed: $($result.Errors -join ' | ')"
  }
  if ([string]$result.Mode -ne "dry-run" -or [string]$result.Verdict -ne "dry_run_observation_complete") {
    return New-CheckOutcome -Passed $false -Detail "unexpected mode/verdict: $($result.Mode)/$($result.Verdict)"
  }
  if (@($script:ShowWindowCalls).Count -ne 0 -or @($script:SetWindowPosCalls).Count -ne 0) {
    return New-CheckOutcome -Passed $false -Detail "MUTATIONS RECORDED IN DRY-RUN"
  }
  if ([int]$result.OperationsPerformed -ne 0 -or @($result.PlannedOperations).Count -ne 0) {
    return New-CheckOutcome -Passed $false -Detail "dry-run planned or performed operations"
  }
  $operationJson = Read-EvidenceJson -EvidenceDirectory $result.EvidencePath -FileName "operation.json"
  if ([bool]$operationJson.performed -or
      [string]$operationJson.reason -ne "apply_not_authorized_dry_run_read_only" -or
      -not [bool]$operationJson.pane_children_operated -eq $false) {
    return New-CheckOutcome -Passed $false -Detail "operation.json does not prove the read-only dry-run"
  }
  return New-CheckOutcome -Passed $true -Detail "default invocation is a read-only dry-run with zero mutations"
}

function Invoke-CheckDryRunWritesCompleteEvidenceSet {
  Set-CoreStubWorld -StateFlagsMode "always_maximized"
  $result = Invoke-StubbedAcceptanceRun -ParameterOverrides @{ Apply = $false }
  if (-not [bool]$result.Succeeded) {
    return New-CheckOutcome -Passed $false -Detail "dry-run failed: $($result.Errors -join ' | ')"
  }
  foreach ($artifactName in @("provenance.json", "display.json", "before.json", "operation.json", "final.json", "SHA256SUMS.txt")) {
    if (-not (Test-Path -LiteralPath (Join-Path $result.EvidencePath $artifactName))) {
      return New-CheckOutcome -Passed $false -Detail "missing artifact '$artifactName'"
    }
  }
  if (Test-Path -LiteralPath (Join-Path $result.EvidencePath "failure.json")) {
    return New-CheckOutcome -Passed $false -Detail "failure.json must not exist on success"
  }
  $provenance = Read-EvidenceJson -EvidenceDirectory $result.EvidencePath -FileName "provenance.json"
  if ([bool]$provenance.screenshot_claimed) {
    return New-CheckOutcome -Passed $false -Detail "provenance claims screenshots"
  }
  if ([int]$provenance.target_executable.pid -ne [int]$script:StubPid) {
    return New-CheckOutcome -Passed $false -Detail "provenance pid mismatch"
  }
  return New-CheckOutcome -Passed $true -Detail "provenance/display/before/operation/final + SHA256SUMS present; no screenshot claim"
}

function Invoke-CheckDryRunCriteriaInformationalOnly {
  Set-CoreStubWorld -StateFlagsMode "always_minimized"
  $result = Invoke-StubbedAcceptanceRun -ParameterOverrides @{ Apply = $false }
  if (-not [bool]$result.Succeeded) {
    return New-CheckOutcome -Passed $false -Detail "dry-run must not fail on unmet acceptance criteria: $($result.Errors -join ' | ')"
  }
  $finalJson = Read-EvidenceJson -EvidenceDirectory $result.EvidencePath -FileName "final.json"
  if ([bool]$finalJson.last_sample_all_criteria_met) {
    return New-CheckOutcome -Passed $false -Detail "expected informational criteria to be unmet in always-minimized world"
  }
  return New-CheckOutcome -Passed $true -Detail "criteria evaluated informationally; unmet criteria do not fail a dry-run"
}

# ---------------------------------------------------------------------------
# Checks: apply happy worlds
# ---------------------------------------------------------------------------

function Invoke-CheckApplyAlreadyCompliantPassesWithZeroOperations {
  Set-CoreStubWorld -StateFlagsMode "always_maximized"
  $result = Invoke-StubbedAcceptanceRun
  if (-not [bool]$result.Succeeded) {
    return New-CheckOutcome -Passed $false -Detail "apply happy world failed: $($result.Errors -join ' | ')"
  }
  if ([string]$result.Verdict -ne "acceptance_passed") {
    return New-CheckOutcome -Passed $false -Detail "verdict '$($result.Verdict)' != acceptance_passed"
  }
  if ([int]$result.OperationsPerformed -ne 0) {
    return New-CheckOutcome -Passed $false -Detail "compliant world should need zero operations"
  }
  if ([int]$result.ConsecutiveStableAchieved -ne 3 -or [int]$result.SamplesCollected -ne 3) {
    return New-CheckOutcome -Passed $false -Detail "expected exactly 3 samples, 3 consecutive; got $($result.SamplesCollected)/$($result.ConsecutiveStableAchieved)"
  }
  if (@($script:DelayCalls).Count -ne 2) {
    return New-CheckOutcome -Passed $false -Detail "expected 2 inter-sample delays, got $(@($script:DelayCalls).Count)"
  }
  foreach ($delayValue in @($script:DelayCalls)) {
    if ([int]$delayValue -lt 200) {
      return New-CheckOutcome -Passed $false -Detail "sample separation below 200ms requested: $delayValue"
    }
  }
  return New-CheckOutcome -Passed $true -Detail "acceptance_passed via 3 consecutive stable samples separated by >=200ms"
}

function Invoke-CheckApplyMinimizedRestoresThenMaximizesInOrder {
  Set-CoreStubWorld -StateFlagsMode "minimized_then_maximized"
  $result = Invoke-StubbedAcceptanceRun
  if (-not [bool]$result.Succeeded) {
    return New-CheckOutcome -Passed $false -Detail "minimized world failed: $($result.Errors -join ' | ')"
  }
  $planned = @($result.PlannedOperations)
  if ($planned.Count -ne 2 -or [string]$planned[0] -ne "sw_restore" -or [string]$planned[1] -ne "sw_maximize") {
    return New-CheckOutcome -Passed $false -Detail "plan mismatch: $($planned -join ',')"
  }
  if ((@($script:ShowWindowCalls) -join ",") -ne "show:9,show:3") {
    return New-CheckOutcome -Passed $false -Detail "mutation order wrong: $(@($script:ShowWindowCalls) -join ',')"
  }
  $operationJson = Read-EvidenceJson -EvidenceDirectory $result.EvidencePath -FileName "operation.json"
  $executed = @($operationJson.executed_operations)
  if ($executed.Count -ne 2 -or -not [bool]$executed[0].pre_change_revalidation.passed -or
      -not [bool]$executed[1].pre_change_revalidation.passed) {
    return New-CheckOutcome -Passed $false -Detail "per-operation revalidation blocks missing or not passed"
  }
  return New-CheckOutcome -Passed $true -Detail "SW_RESTORE then SW_MAXIMIZE, each behind a passed revalidation"
}

function Invoke-CheckApplyOffMonitorRepositionsWithExactRect {
  Set-CoreStubWorld -StateFlagsMode "small_then_maximized" -NearestMonitorSequence @(
    [long]$script:OtherMonitorHandle,
    [long]$script:TargetMonitorHandle
  )
  $result = Invoke-StubbedAcceptanceRun
  if (-not [bool]$result.Succeeded) {
    return New-CheckOutcome -Passed $false -Detail "off-monitor world failed: $($result.Errors -join ' | ')"
  }
  $planned = @($result.PlannedOperations)
  if ($planned.Count -ne 2 -or [string]$planned[0] -ne "reposition_to_target_monitor" -or [string]$planned[1] -ne "sw_maximize") {
    return New-CheckOutcome -Passed $false -Detail "plan mismatch: $($planned -join ',')"
  }
  # Work area 3840x2160 -> two-thirds rect 2560x1440 centered at (640,360).
  if ((@($script:SetWindowPosCalls) -join ",") -ne "setpos:640,360,2560,1440") {
    return New-CheckOutcome -Passed $false -Detail "SetWindowPos args wrong: $(@($script:SetWindowPosCalls) -join ',')"
  }
  if ((@($script:ShowWindowCalls) -join ",") -ne "show:3") {
    return New-CheckOutcome -Passed $false -Detail "unexpected ShowWindow calls: $(@($script:ShowWindowCalls) -join ',')"
  }
  return New-CheckOutcome -Passed $true -Detail "cross-monitor reposition used exact centered work-area rect then maximized"
}

# ---------------------------------------------------------------------------
# Checks: observed identity mismatches (no mutations)
# ---------------------------------------------------------------------------

function Invoke-CheckObservedIdentityMismatchesRejectedBeforeMutation {
  Set-CoreStubWorld
  $cases = @(
    @{ Overrides = @{ ExpectedSha256 = $script:BadSha256 }; Token = "observed sha256"; Name = "sha mismatch" },
    @{ Overrides = @{ ExpectedProductVersion = $script:BadVersion }; Token = "observed product version"; Name = "version mismatch" },
    @{ Overrides = @{ ExpectedGitHead = $script:BadHead }; Token = "observed git HEAD"; Name = "head mismatch" }
  )
  foreach ($case in $cases) {
    $result = Invoke-StubbedAcceptanceRun -ParameterOverrides $case.Overrides
    $outcome = Assert-ResultFailedAtStage -Result $result -ExpectedStage "target_resolution" `
      -ErrorTokens @($case.Token, "authorization identity mismatch") -FailDetail $case.Name
    if (-not $outcome.Passed) { return $outcome }
    if (@($script:ShowWindowCalls).Count -ne 0 -or @($script:SetWindowPosCalls).Count -ne 0) {
      return New-CheckOutcome -Passed $false -Detail "$($case.Name): mutations reached despite identity mismatch"
    }
  }
  return New-CheckOutcome -Passed $true -Detail "wrong observed sha/version/head each fail before any mutation"
}

# ---------------------------------------------------------------------------
# Checks: pre-change revalidation aborts remaining operations
# ---------------------------------------------------------------------------

function Invoke-CheckPreChangeRevalidationAbortsRemainingOps {
  # Each scenario starts minimized (plan: sw_restore, sw_maximize), lets the
  # FIRST operation succeed, then flips one fact so the SECOND operation's
  # revalidation must abort before any further mutation.
  $scenarios = @(
    @{
      Name = "hash flips after first op"
      Configure = {
        # Calls before the first operation's revalidation: target_resolution
        # proof + before-sample deep proof; the flip must hit only the SECOND
        # operation's revalidation.
        $script:ShaGoodUntil = 3
      }
      ForbiddenSpyToken = "show:3"
      ErrorTokens = @("observed sha256")
      SpyMustContain = "show:9"
    },
    @{
      Name = "version flips before second op"
      Configure = {
        $script:VersionGoodUntil = 3
      }
      ForbiddenSpyToken = "show:3"
      ErrorTokens = @("observed product version")
      SpyMustContain = "show:9"
    },
    @{
      Name = "ownership changes before second op"
      Configure = {
        $script:OwnerGoodUntil = 3
      }
      ForbiddenSpyToken = "show:3"
      ErrorTokens = @("ownership changed before sw_maximize")
      SpyMustContain = "show:9"
    },
    @{
      Name = "HWND vanishes before second op"
      Configure = {
        $script:AliveGoodUntil = 4
      }
      ForbiddenSpyToken = "show:3"
      ErrorTokens = @("vanished before sw_maximize")
      SpyMustContain = "show:9"
    },
    @{
      Name = "window stops responding before second op"
      Configure = {
        $script:HungGoodUntil = 2
      }
      ForbiddenSpyToken = "show:3"
      ErrorTokens = @("not responding before sw_maximize")
      SpyMustContain = "show:9"
    },
    @{
      Name = "window leaves target monitor before second op"
      Configure = {
        $script:MonForWinSequence = @([long]$script:TargetMonitorHandle, [long]$script:TargetMonitorHandle, [long]$script:OtherMonitorHandle)
      }
      ForbiddenSpyToken = "show:3"
      ErrorTokens = @("no longer on the target monitor")
      SpyMustContain = "show:9"
    }
  )

  foreach ($scenario in $scenarios) {
    Set-CoreStubWorld -StateFlagsMode "minimized_then_maximized"
    & $scenario.Configure
    $result = Invoke-StubbedAcceptanceRun
    if ([bool]$result.Succeeded) {
      return New-CheckOutcome -Passed $false -Detail "$($scenario.Name): run unexpectedly succeeded"
    }
    if ((@($script:ShowWindowCalls) -join ",").Contains($scenario.ForbiddenSpyToken)) {
      return New-CheckOutcome -Passed $false -Detail "$($scenario.Name): mutation executed after failed revalidation"
    }
    if (-not ((@($script:ShowWindowCalls) -join ",").Contains($scenario.SpyMustContain))) {
      return New-CheckOutcome -Passed $false -Detail "$($scenario.Name): first expected mutation missing (world misconfigured)"
    }
    $foundToken = $false
    foreach ($errorMessage in @($result.Errors)) {
      foreach ($token in @($scenario.ErrorTokens)) {
        if ($errorMessage.Contains($token)) { $foundToken = $true }
      }
    }
    if (-not $foundToken) {
      return New-CheckOutcome -Passed $false -Detail "$($scenario.Name): expected abort token missing; errors: $($result.Errors -join ' | ')"
    }
    $failurePath = Join-Path $result.EvidencePath "failure.json"
    if (-not (Test-Path -LiteralPath $failurePath)) {
      return New-CheckOutcome -Passed $false -Detail "$($scenario.Name): failure.json not retained"
    }
  }
  return New-CheckOutcome -Passed $true -Detail "6 flip scenarios: revalidation aborted remaining operations, evidence retained"
}

# ---------------------------------------------------------------------------
# Checks: target monitor resolution
# ---------------------------------------------------------------------------

function Invoke-CheckZeroMatchingMonitorsFailsApplyButNotDryRun {
  Set-CoreStubWorld -MonitorIdentities @(
    (New-StubMonitorIdentity -HandleValue $script:TargetMonitorHandle -Description "DELL-U2415")
  )
  $applyResult = Invoke-StubbedAcceptanceRun
  $applyOutcome = Assert-ResultFailedAtStage -Result $applyResult `
    -ExpectedStage "display_and_window_resolution" `
    -ErrorTokens @("0 monitors match") -FailDetail "missing monitor (apply)"
  if (-not $applyOutcome.Passed) { return $applyOutcome }

  Set-CoreStubWorld -MonitorIdentities @(
    (New-StubMonitorIdentity -HandleValue $script:TargetMonitorHandle -Description "DELL-U2415")
  )
  $dryResult = Invoke-StubbedAcceptanceRun -ParameterOverrides @{ Apply = $false }
  if (-not [bool]$dryResult.Succeeded) {
    return New-CheckOutcome -Passed $false -Detail "dry-run must tolerate zero tuple matches informationally"
  }
  $displayJson = Read-EvidenceJson -EvidenceDirectory $dryResult.EvidencePath -FileName "display.json"
  if ([int]$displayJson.selection.match_count -ne 0) {
    return New-CheckOutcome -Passed $false -Detail "display.json match_count should be 0"
  }
  return New-CheckOutcome -Passed $true -Detail "zero matching monitors: apply fails closed; dry-run records informationally"
}

function Invoke-CheckDuplicateMonitorTupleAmbiguousInApply {
  Set-CoreStubWorld -MonitorIdentities @(
    (New-StubMonitorIdentity -HandleValue $script:TargetMonitorHandle),
    (New-StubMonitorIdentity -HandleValue ([long]7003))
  )
  $applyResult = Invoke-StubbedAcceptanceRun
  $applyOutcome = Assert-ResultFailedAtStage -Result $applyResult `
    -ExpectedStage "display_and_window_resolution" `
    -ErrorTokens @("2 monitors match", "ambiguous") -FailDetail "duplicate tuples (apply)"
  if (-not $applyOutcome.Passed) { return $applyOutcome }
  return New-CheckOutcome -Passed $true -Detail $applyOutcome.Detail
}

function Invoke-CheckDuplicateTupleToleratedInformationallyInDryRun {
  Set-CoreStubWorld -MonitorIdentities @(
    (New-StubMonitorIdentity -HandleValue $script:TargetMonitorHandle),
    (New-StubMonitorIdentity -HandleValue ([long]7003))
  )
  $dryResult = Invoke-StubbedAcceptanceRun -ParameterOverrides @{ Apply = $false }
  if (-not [bool]$dryResult.Succeeded) {
    return New-CheckOutcome -Passed $false -Detail "dry-run duplicate-tuple world should not fail"
  }
  $displayJson = Read-EvidenceJson -EvidenceDirectory $dryResult.EvidencePath -FileName "display.json"
  if ([int]$displayJson.selection.match_count -ne 2 -or -not [bool]$displayJson.selection.ambiguous_duplicate_tuples_rejected_in_apply_mode) {
    return New-CheckOutcome -Passed $false -Detail "display.json does not record the ambiguity policy"
  }
  return New-CheckOutcome -Passed $true -Detail "duplicate tuples recorded as ambiguous-for-apply in dry-run display.json"
}

function Invoke-CheckUnavailableMonitorDescriptionRecordedAndFailsClosed {
  Set-CoreStubWorld -MonitorIdentities @(
    (New-StubMonitorIdentity -HandleValue $script:TargetMonitorHandle -Description "")
  )
  $applyResult = Invoke-StubbedAcceptanceRun
  $applyOutcome = Assert-ResultFailedAtStage -Result $applyResult `
    -ExpectedStage "display_and_window_resolution" `
    -ErrorTokens @("0 monitors match") -FailDetail "unavailable description (apply)"
  if (-not $applyOutcome.Passed) { return $applyOutcome }
  return New-CheckOutcome -Passed $true -Detail "empty description cannot satisfy the identity tuple; apply fails closed"
}

# ---------------------------------------------------------------------------
# Checks: stability sampling success/failure
# ---------------------------------------------------------------------------

function Invoke-CheckInstabilityResetsConsecutiveCountAndExhausts {
  Set-CoreStubWorld -StateFlagsMode "unstable_every_third"
  $result = Invoke-StubbedAcceptanceRun -ParameterOverrides @{ MaxSampleAttempts = 6 }
  if ([bool]$result.Succeeded) {
    return New-CheckOutcome -Passed $false -Detail "unstable world unexpectedly passed acceptance"
  }
  if ([string]$result.Verdict -ne "acceptance_failed_stability_not_achieved") {
    return New-CheckOutcome -Passed $false -Detail "verdict '$($result.Verdict)' != acceptance_failed_stability_not_achieved"
  }
  if ([int]$result.SamplesCollected -ne 6 -or [int]$result.ConsecutiveStableAchieved -ge 3) {
    return New-CheckOutcome -Passed $false -Detail (
      "reset semantics wrong: attempts={0} consecutive={1}" -f $result.SamplesCollected, $result.ConsecutiveStableAchieved)
  }
  $failurePath = Join-Path $result.EvidencePath "failure.json"
  if (-not (Test-Path -LiteralPath $failurePath)) {
    return New-CheckOutcome -Passed $false -Detail "stability failure did not retain failure.json"
  }
  $finalJson = Read-EvidenceJson -EvidenceDirectory $result.EvidencePath -FileName "final.json"
  if (@($finalJson.stability.samples).Count -ne 6) {
    return New-CheckOutcome -Passed $false -Detail "final.json should retain all 6 samples"
  }
  $metFlags = @(@($finalJson.stability.samples) | ForEach-Object { [bool]$_.all_criteria_met })
  $sawReset = $false
  for ($flagIndex = 1; $flagIndex -lt $metFlags.Count; $flagIndex++) {
    if (-not $metFlags[$flagIndex] -and $metFlags[$flagIndex - 1]) { $sawReset = $true }
  }
  if (-not $sawReset) {
    return New-CheckOutcome -Passed $false -Detail "no unstable sample ever followed a met sample; reset path unproven"
  }
  return New-CheckOutcome -Passed $true -Detail "unstable sample resets the count; exhaustion fails with retained evidence"
}

# ---------------------------------------------------------------------------
# Checks: evidence directory safety
# ---------------------------------------------------------------------------

function Invoke-CheckUnsafeSlugsRejected {
  New-TestSandbox
  $rootPath = New-FreshEvidenceRoot
  # NOTE: an empty slug is intentionally NOT in this list; it auto-generates a
  # timestamped name by contract (proven separately in the auto-create check).
  foreach ($badSlug in @("../evil", "a\b", "..", ".", ".hidden", "slug:d", "UPPERCASE", "a b", "selftest.", "-no-leading-dash")) {
    $outcome = Assert-ThrowContains `
      -Action { New-AcceptanceEvidenceDirectory -RootPath $rootPath -Slug $badSlug } `
      -MustContain @("unsafe evidence slug") `
      -FailDetail "slug '$badSlug'"
    if (-not $outcome.Passed) { return $outcome }
  }
  return New-CheckOutcome -Passed $true -Detail "traversal/relative/hidden/reserved-format/uppercase/space/trailing-dot slugs all rejected"
}

function Invoke-CheckReservedDeviceNamesRejected {
  New-TestSandbox
  $rootPath = New-FreshEvidenceRoot
  foreach ($reservedName in @("CON", "nul", "com1", "lpt2")) {
    $outcome = Assert-ThrowContains `
      -Action { New-AcceptanceEvidenceDirectory -RootPath $rootPath -Slug $reservedName } `
      -MustContain @("unsafe evidence slug") `
      -FailDetail "reserved name '$reservedName'"
    if (-not $outcome.Passed) { return $outcome }
  }
  return New-CheckOutcome -Passed $true -Detail "Windows reserved device names rejected case-insensitively"
}

function Invoke-CheckExistingCandidateNeverReused {
  New-TestSandbox
  $rootPath = New-FreshEvidenceRoot
  [void](New-Item -ItemType Directory -Path (Join-Path $rootPath "selftest") -Force)
  $outcome = Assert-ThrowContains `
    -Action { New-AcceptanceEvidenceDirectory -RootPath $rootPath -Slug "selftest" } `
    -MustContain @("refusing to reuse or overwrite existing evidence path") `
    -FailDetail "existing candidate directory"
  if (-not $outcome.Passed) { return $outcome }
  return New-CheckOutcome -Passed $true -Detail "pre-existing candidate never reused, merged, or overwritten"
}

function Invoke-CheckJunctionInAncestryRejected {
  New-TestSandbox
  $junctionTarget = Join-Path $script:SandboxRoot ("jt-" + [Guid]::NewGuid().ToString("N"))
  $junctionPath = Join-Path $script:SandboxRoot ("jn-" + [Guid]::NewGuid().ToString("N"))
  [void](New-Item -ItemType Directory -Path $junctionTarget -Force)
  try {
    [void](New-Item -ItemType Junction -Path $junctionPath -Value $junctionTarget)
    $outcome = Assert-ThrowContains `
      -Action { New-AcceptanceEvidenceDirectory -RootPath $junctionPath -Slug "selftest" } `
      -MustContain @("reparse point detected in evidence path ancestry") `
      -FailDetail "junction root"
    if (-not $outcome.Passed) { return $outcome }
    return New-CheckOutcome -Passed $true -Detail "junction anywhere in the evidence ancestry is rejected"
  } finally {
    if (Test-Path -LiteralPath $junctionPath) {
      Remove-Item -LiteralPath $junctionPath -Force
    }
  }
}

function Invoke-CheckOwnerSidDecisionMatrix {
  $currentUserSid = [string][Security.Principal.WindowsIdentity]::GetCurrent().User.Value
  $acceptedCases = @($currentUserSid, "S-1-5-32-544", "S-1-5-18")
  foreach ($acceptedSid in $acceptedCases) {
    if (-not (Test-AcceptanceEvidenceOwnerSidAccepted -OwnerSidValue $acceptedSid)) {
      return New-CheckOutcome -Passed $false -Detail "owner SID '$acceptedSid' should be accepted"
    }
  }
  foreach ($rejectedSid in @("S-1-1-0", "S-1-5-32-545")) {
    if (Test-AcceptanceEvidenceOwnerSidAccepted -OwnerSidValue $rejectedSid) {
      return New-CheckOutcome -Passed $false -Detail "owner SID '$rejectedSid' should be rejected"
    }
  }
  return New-CheckOutcome -Passed $true -Detail "only current user / Administrators / SYSTEM accepted as evidence parent owner"
}

function Invoke-CheckEvidenceRootAutoCreatedWhenMissing {
  New-TestSandbox
  $deepRoot = Join-Path $script:SandboxRoot "deep\a\b\c\evidence-root"
  $createdPath = New-AcceptanceEvidenceDirectory -RootPath $deepRoot -Slug "selftest"
  if (-not (Test-Path -LiteralPath $createdPath -PathType Container)) {
    return New-CheckOutcome -Passed $false -Detail "candidate directory was not created under a fresh root"
  }
  if (@(Get-ChildItem -LiteralPath $createdPath -Force).Count -ne 0) {
    return New-CheckOutcome -Passed $false -Detail "freshly created candidate is not empty"
  }
  return New-CheckOutcome -Passed $true -Detail "evidence root chain auto-created; new empty direct child proven"
}

function Invoke-CheckSha256SumsIntegrityOnSuccess {
  Set-CoreStubWorld -StateFlagsMode "always_maximized"
  $result = Invoke-StubbedAcceptanceRun
  if (-not [bool]$result.Succeeded) {
    return New-CheckOutcome -Passed $false -Detail "happy run failed: $($result.Errors -join ' | ')"
  }
  $sumsLines = @(Get-Content -LiteralPath (Join-Path $result.EvidencePath "SHA256SUMS.txt"))
  $listedNames = @()
  foreach ($line in $sumsLines) {
    $parts = $line -split "  ", 2
    if ($parts.Count -ne 2) { return New-CheckOutcome -Passed $false -Detail "malformed sums line: $line" }
    $listedNames += $parts[1]
    $actualHash = (Get-FileHash -LiteralPath (Join-Path $result.EvidencePath $parts[1]) -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actualHash -ne $parts[0].ToLowerInvariant()) {
      return New-CheckOutcome -Passed $false -Detail "hash mismatch for $($parts[1])"
    }
  }
  $actualFiles = @(Get-ChildItem -LiteralPath $result.EvidencePath | ForEach-Object { $_.Name })
  foreach ($actualFile in $actualFiles) {
    if ($actualFile -eq "SHA256SUMS.txt") { continue }  # checksum file excludes itself by convention
    if ($listedNames -notcontains $actualFile) {
      return New-CheckOutcome -Passed $false -Detail "$actualFile missing from SHA256SUMS.txt"
    }
  }
  if ($listedNames -contains "SHA256SUMS.txt") {
    return New-CheckOutcome -Passed $false -Detail "SHA256SUMS.txt must not list itself"
  }
  return New-CheckOutcome -Passed $true -Detail "SHA256SUMS.txt covers every artifact with correct hashes"
}

# ---------------------------------------------------------------------------
# Checks: static contract of the runner source
# ---------------------------------------------------------------------------

function Invoke-CheckStaticContractOfRunnerSource {
  $runnerText = Get-Content -LiteralPath $script:RunnerPath -Raw

  foreach ($forbiddenToken in @(
    @{ Token = "Stop-Process"; Why = "process termination prohibited" },
    @{ Token = "taskkill"; Why = "process termination prohibited" },
    @{ Token = "TerminateProcess"; Why = "process termination prohibited" },
    @{ Token = ".Kill("; Why = "process termination prohibited" },
    @{ Token = "CloseMainWindow"; Why = "process termination adjacent" },
    @{ Token = "EnumChildWindows"; Why = "pane children are not operated this tranche" },
    @{ Token = "PrintWindow"; Why = "screen capture prohibited" },
    @{ Token = "BitBlt"; Why = "screen capture prohibited" },
    @{ Token = "CopyFromScreen"; Why = "screen capture prohibited" },
    @{ Token = "Invoke-WebRequest"; Why = "network prohibited" },
    @{ Token = "Invoke-RestMethod"; Why = "network prohibited" },
    @{ Token = "WebClient"; Why = "network prohibited" },
    @{ Token = "HttpClient"; Why = "network prohibited" },
    @{ Token = "TcpClient"; Why = "network prohibited" },
    @{ Token = "SendInput"; Why = "input injection prohibited" },
    @{ Token = "keybd_event"; Why = "input injection prohibited" },
    @{ Token = "mouse_event"; Why = "input injection prohibited" },
    @{ Token = "SetForegroundWindow"; Why = "foreground stealing prohibited" },
    @{ Token = "BringWindowToTop"; Why = "foreground stealing prohibited" },
    @{ Token = "AttachThreadInput"; Why = "thread attachment prohibited" },
    @{ Token = "MoveWindow"; Why = "only SetWindowPos with NOZORDER|NOACTIVATE allowed" },
    @{ Token = "PostMessage"; Why = "message injection prohibited" },
    @{ Token = "SendMessage"; Why = "message injection prohibited" },
    @{ Token = "Start-Process"; Why = "process launch prohibited" },
    @{ Token = "Out-File"; Why = "artifact writes go through WriteAllText only" },
    @{ Token = "Set-Content"; Why = "artifact writes go through WriteAllText only" },
    @{ Token = "Add-Content"; Why = "artifact writes go through WriteAllText only" }
  )) {
    if ($runnerText.Contains($forbiddenToken.Token)) {
      return New-CheckOutcome -Passed $false -Detail (
        "forbidden token '{0}' present ({1})" -f $forbiddenToken.Token, $forbiddenToken.Why)
    }
  }

  foreach ($requiredToken in @(
    "-Apply", "MSI3DD2", "3840x2160", "3840x2088", "2560x1392",
    "SHA256SUMS", "failure.json", "provenance.json", "display.json",
    "before.json", "operation.json", "final.json", "ReparsePoint",
    "SW_RESTORE", "SW_MAXIMIZE", "ShowWindow", "SetWindowPos",
    "IsHungAppWindow", "GetWindowThreadProcessId",
    "observe-syndocal-window-geometry.ps1", "NO process termination"
  )) {
    if (-not $runnerText.Contains($requiredToken)) {
      return New-CheckOutcome -Passed $false -Detail "required token '$requiredToken' missing from runner"
    }
  }

  $parseErrors = $null
  $tokens = $null
  $ast = [System.Management.Automation.Language.Parser]::ParseFile(
    $script:RunnerPath, [ref]$tokens, [ref]$parseErrors)
  if (@($parseErrors).Count -gt 0) {
    $firstError = @($parseErrors)[0]
    return New-CheckOutcome -Passed $false -Detail (
      "parse error at {0}:{1}: {2}" -f $firstError.Extent.StartLineNumber,
      $firstError.Extent.StartColumnNumber, $firstError.Message)
  }

  # Host-agnostic exit detection: Windows PowerShell parses "exit" as a
  # CommandAst, while newer pwsh versions use a dedicated ExitStatementAst.
  $exitCommands = @(
    $ast.FindAll({ param($node)
      (($node -is [System.Management.Automation.Language.CommandAst] -and
        $node.GetCommandName() -eq "exit") -or
       ($node.GetType().Name -eq "ExitStatementAst")) }, $true)
  )
  $mainFunction = @(
    $ast.FindAll({ param($node) $node -is [System.Management.Automation.Language.FunctionDefinitionAst] -and
      $node.Name -eq "Invoke-AcceptanceMain" }, $true)
  )[0]
  if ($null -eq $mainFunction) {
    return New-CheckOutcome -Passed $false -Detail "Invoke-AcceptanceMain definition not found"
  }
  if (@($exitCommands).Count -eq 0) {
    return New-CheckOutcome -Passed $false -Detail "expected exit statements inside Invoke-AcceptanceMain"
  }
  foreach ($exitCommand in @($exitCommands)) {
    $offset = $exitCommand.Extent.StartOffset
    if ($offset -lt $mainFunction.Extent.StartOffset -or $offset -gt $mainFunction.Extent.EndOffset) {
      return New-CheckOutcome -Passed $false -Detail (
        "exit statement outside Invoke-AcceptanceMain at offset $offset; functions must return results, not exit")
    }
  }
  return New-CheckOutcome -Passed $true -Detail (
    "static contract holds; zero parse errors; exits confined to main wrapper; no capture/network/termination/child-window APIs")
}

function Invoke-CheckObserverContractsAvailableThroughDotSource {
  foreach ($contractFunctionName in @(
    "Select-SyndocalTargetProcessId",
    "Get-SyndocalCandidateProcessIds",
    "Test-WindowNotHung",
    "Get-ObservationWindowStateFlags"
  )) {
    if ($null -eq (Get-Command -Name $contractFunctionName -ErrorAction SilentlyContinue)) {
      return New-CheckOutcome -Passed $false -Detail "observer contract function '$contractFunctionName' unavailable after dot-sourcing the runner"
    }
  }
  return New-CheckOutcome -Passed $true -Detail "observer fail-closed machinery inherited verbatim through dot-source"
}

# ---------------------------------------------------------------------------
# All checks
# ---------------------------------------------------------------------------

function Invoke-AllFocusedChecks {
  $results = [System.Collections.Generic.List[object]]::new()
  New-TestSandbox
  [void](New-SyntheticTargetExecutable)
  $savedSeams = Save-Seams
  try {
    $checks = @(
      @{ Name = "apply without expectations fails closed before any seam"; Run = { Invoke-CheckApplyWithoutExpectationsFailsClosed } },
      @{ Name = "each missing expectation field rejected with -Apply"; Run = { Invoke-CheckApplyEachMissingFieldRejected } },
      @{ Name = "malformed sha/head/path/leaf formats rejected"; Run = { Invoke-CheckApplyMalformedFormatsRejected } },
      @{ Name = "off-pin monitor identifier/resolution/dpi rejected"; Run = { Invoke-CheckApplyOffPinMonitorTupleRejected } },
      @{ Name = "sample interval and attempts floors enforced"; Run = { Invoke-CheckSamplingFloorsEnforced } },
      @{ Name = "dry-run default performs zero mutations"; Run = { Invoke-CheckDryRunDefaultPerformsZeroMutations } },
      @{ Name = "dry-run writes complete evidence set without failure.json or screenshot claims"; Run = { Invoke-CheckDryRunWritesCompleteEvidenceSet } },
      @{ Name = "dry-run criteria informational only"; Run = { Invoke-CheckDryRunCriteriaInformationalOnly } },
      @{ Name = "compliant world passes acceptance with zero operations"; Run = { Invoke-CheckApplyAlreadyCompliantPassesWithZeroOperations } },
      @{ Name = "minimized world restores then maximizes behind revalidations"; Run = { Invoke-CheckApplyMinimizedRestoresThenMaximizesInOrder } },
      @{ Name = "off-monitor world repositions with exact centered rect"; Run = { Invoke-CheckApplyOffMonitorRepositionsWithExactRect } },
      @{ Name = "observed sha/version/head mismatches fail before mutation"; Run = { Invoke-CheckObservedIdentityMismatchesRejectedBeforeMutation } },
      @{ Name = "pre-change revalidation aborts remaining operations (6 flip scenarios)"; Run = { Invoke-CheckPreChangeRevalidationAbortsRemainingOps } },
      @{ Name = "zero matching monitors fail apply, recorded informationally in dry-run"; Run = { Invoke-CheckZeroMatchingMonitorsFailsApplyButNotDryRun } },
      @{ Name = "duplicate monitor tuple ambiguous in apply mode"; Run = { Invoke-CheckDuplicateMonitorTupleAmbiguousInApply } },
      @{ Name = "duplicate tuple policy recorded in dry-run display evidence"; Run = { Invoke-CheckDuplicateTupleToleratedInformationallyInDryRun } },
      @{ Name = "unavailable monitor description cannot satisfy identity tuple"; Run = { Invoke-CheckUnavailableMonitorDescriptionRecordedAndFailsClosed } },
      @{ Name = "instability resets consecutive count and exhaustion fails retaining evidence"; Run = { Invoke-CheckInstabilityResetsConsecutiveCountAndExhausts } },
      @{ Name = "traversal/unsafe slugs rejected"; Run = { Invoke-CheckUnsafeSlugsRejected } },
      @{ Name = "reserved device names rejected"; Run = { Invoke-CheckReservedDeviceNamesRejected } },
      @{ Name = "existing candidate directory never reused"; Run = { Invoke-CheckExistingCandidateNeverReused } },
      @{ Name = "junction in evidence ancestry rejected"; Run = { Invoke-CheckJunctionInAncestryRejected } },
      @{ Name = "owner SID decision matrix"; Run = { Invoke-CheckOwnerSidDecisionMatrix } },
      @{ Name = "evidence root auto-created; fresh empty direct child proven"; Run = { Invoke-CheckEvidenceRootAutoCreatedWhenMissing } },
      @{ Name = "SHA256SUMS integrity on success"; Run = { Invoke-CheckSha256SumsIntegrityOnSuccess } },
      @{ Name = "static runner contract: forbidden/required tokens, parse, exit confinement"; Run = { Invoke-CheckStaticContractOfRunnerSource } },
      @{ Name = "observer contracts available through dot-source"; Run = { Invoke-CheckObserverContractsAvailableThroughDotSource } }
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
    Remove-TestSandbox
  }
  return @($results)
}

if ($script:RunningUnderPester) {
  Describe "run-syndocal-native-4k-acceptance focused checks" {
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
