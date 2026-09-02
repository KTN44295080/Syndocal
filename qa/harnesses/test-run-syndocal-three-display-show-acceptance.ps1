# Deterministic seam tests for run-syndocal-three-display-show-acceptance.ps1.
# No real Syndocal process, HWND, monitor, input, network, or hardware API is
# queried or mutated: every such runner seam is replaced before a test runs.
# The only real writes are temporary evidence files under this test's unique
# temporary directory, removed in finally after the tests finish.

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$script:RunnerPath = Join-Path $PSScriptRoot "run-syndocal-three-display-show-acceptance.ps1"
. $script:RunnerPath

$script:SandboxRoot = Join-Path ([IO.Path]::GetTempPath()) ("syndocal-three-display-selftest-" + [Guid]::NewGuid().ToString("N"))
$script:CheckoutRoot = Join-Path $script:SandboxRoot "checkout"
$script:ExpectedPath = Join-Path $script:CheckoutRoot "target\release\syndocal.exe"
$script:GoodPid = [uint32]4242
$script:WrongPid = [uint32]4243
$script:GoodHash = "29045BC40227F823E0A2259113E2BECC246B665AEC161E5A14111D9CB0E4EA1C"
$script:WrongHash = ("cd" * 32)
$script:GoodSize = [uint64]62435840
$script:WrongSize = [uint64]61691905
$script:GoodVersion = "1.2.0-alpha.60"
$script:StaleVersion = "1.2.0-alpha.41"
$script:WrongVersion = "9.9.9-wrong"
$script:GoodArtifactHead = "dabbcaec8157b7c4f648af82516ee607394a7c90"
$script:GoodHead = "dabbcaec8157b7c4f648af82516ee607394a7c90"
$script:GoodDescendantHead = ("6" * 40)
$script:GoodShowArtifactHead = ("a" * 40)
$script:GoodShowEvidenceHead = ("b" * 40)
$script:WrongHead = ("dead" * 10)
$script:GoodBranch = "codex/syndocal-v1.2"
$script:WrongBranch = "codex/wrong-current-branch"
$script:SourceAncestorResult = $true
$script:DelayCalls = [System.Collections.Generic.List[int]]::new()
$script:EditorIdentity = "\\?\DISPLAY#TEST_EDITOR#A#{11111111-1111-1111-1111-111111111111}"
$script:LedIdentity = "\\?\DISPLAY#TEST_LED#B#{22222222-2222-2222-2222-222222222222}"
$script:ProjectorIdentity = "\\?\DISPLAY#TEST_PROJECTOR#C#{33333333-3333-3333-3333-333333333333}"
$script:StaleIdentity = "\\?\DISPLAY#TEST_STALE#Z#{44444444-4444-4444-4444-444444444444}"

$script:SeamNames = @(
  "Get-SyndocalCandidateProcesses",
  "Get-ExecutableSha256",
  "Get-ExecutableByteSize",
  "Get-ExecutableProductVersion",
  "Resolve-ThreeDisplayGitHead",
  "Resolve-ThreeDisplayGitBranch",
  "Test-ThreeDisplayGitAncestor",
  "Test-ThreeDisplayCheckoutClean",
  "Get-ThreeDisplayMonitorInventory",
  "Get-ThreeDisplayTopLevelWindows",
  "Get-ThreeDisplayProcessRecord",
  "New-ThreeDisplayCdpClientWebSocket",
  "Get-ThreeDisplayCdpTransportObservation",
  "Get-ThreeDisplayWindowMetrics",
  "Invoke-WithThreeDisplayPhysicalDpiContext",
  "Invoke-ThreeDisplayWindowMaximize",
  "Invoke-ThreeDisplaySampleDelay",
  "Get-ThreeDisplayWindowTitle",
  "Test-ThreeDisplayNativeIsWindow",
  "Get-ThreeDisplayNativeOwnerProcessId",
  "Invoke-ThreeDisplayNativeShowWindow",
  "Invoke-ThreeDisplayShowAsioArtifactVerification",
  "Test-ThreeDisplaySingleLinkFile"
)

function Save-TestSeams {
  $saved = @{}
  foreach ($name in $script:SeamNames) { $saved[$name] = (Get-Item -LiteralPath "function:$name").ScriptBlock }
  return $saved
}

function Restore-TestSeams {
  param([hashtable]$Saved)
  foreach ($name in $Saved.Keys) { Set-Item -LiteralPath "function:$name" -Value $Saved[$name] }
}

function Set-TestSeam {
  param([string]$Name, [scriptblock]$Body)
  Set-Item -LiteralPath "function:$Name" -Value $Body
}

function Install-NativeMaximizeSeams {
  # Replaces only the narrow native seams around the maximize body, never
  # Invoke-ThreeDisplayWindowMaximize itself, so each check executes the real
  # runner body against synthetic HWNDs.  Good-world fixtures install a
  # throwing guard in place of the real maximize body and earlier checks may
  # have left that guard installed, so every installation first restores the
  # pristine runner body saved before any seam was replaced.
  param(
    [bool]$Alive = $true,
    [uint32]$OwnerPid = $script:GoodPid,
    [string]$Title = "Syndocal",
    [bool]$ShowResult = $true,
    [long]$Handle = 11
  )
  Set-Item -LiteralPath "function:Invoke-ThreeDisplayWindowMaximize" -Value $script:SavedRunnerFunctions["Invoke-ThreeDisplayWindowMaximize"]
  $script:NativeWorld = @{
    iswindow = @{ $Handle = $Alive }
    owner = @{ $Handle = $OwnerPid }
    title = @{ $Handle = $Title }
    showcalls = [System.Collections.Generic.List[object]]::new()
    showresult = $ShowResult
  }
  Set-TestSeam "Test-ThreeDisplayNativeIsWindow" { param($Handle) [bool]$script:NativeWorld.iswindow[[long]$Handle] }
  Set-TestSeam "Get-ThreeDisplayNativeOwnerProcessId" { param($Handle) [uint32]$script:NativeWorld.owner[[long]$Handle] }
  Set-TestSeam "Get-ThreeDisplayWindowTitle" { param($Handle) [string]$script:NativeWorld.title[[long]$Handle] }
  Set-TestSeam "Invoke-ThreeDisplayNativeShowWindow" {
    param($Handle, $Command)
    [void]$script:NativeWorld.showcalls.Add([pscustomobject]@{ handle = [long]$Handle; command = [int]$Command })
    return [bool]$script:NativeWorld.showresult
  }
}

function Restore-NativeMaximizeSeams {
  # Restores only the four narrow native seams; the maximize body is owned by
  # the Install-GoodWorldSeams guard, which every caller reinstalls first.
  foreach ($name in @("Get-ThreeDisplayWindowTitle", "Test-ThreeDisplayNativeIsWindow", "Get-ThreeDisplayNativeOwnerProcessId", "Invoke-ThreeDisplayNativeShowWindow")) {
    Set-Item -LiteralPath "function:$name" -Value $script:SavedRunnerFunctions[$name]
  }
}

function New-TestMonitor {
  param(
    [string]$Identity,
    [string]$Device,
    [long]$Handle,
    [int]$Width,
    [int]$Height,
    [int]$Dpi = 96,
    [int]$Left = 0,
    [int]$Top = 0
  )
  $right = $Left + $Width
  $bottom = $Top + $Height
  [pscustomobject]@{
    stable_identity = $Identity; device_name = $Device; friendly_name = $Identity
    monitor_handle_decimal = $Handle; connected = $true; effective_dpi = $Dpi
    physical_bounds = [pscustomobject]@{ left = $Left; top = $Top; right = $right; bottom = $bottom; width = $Width; height = $Height }
    work_area = [pscustomobject]@{ left = $Left; top = $Top; right = $right; bottom = $bottom; width = $Width; height = $Height }
  }
}

function New-TestMetrics {
  param([long]$Handle, [string]$Title, [uint32]$OwnerPid, $Monitor, [int]$Width, [int]$Height, [bool]$Maximized = $true, [bool]$Minimized = $false, [bool]$Responding = $true)
  $left = [int]$Monitor.physical_bounds.left
  $top = [int]$Monitor.physical_bounds.top
  $right = $left + $Width
  $bottom = $top + $Height
  [pscustomobject]@{
    handle_decimal = $Handle; title = $Title; owner_pid = $OwnerPid; alive = $true; visible = $true
    responding = $Responding; minimized = $Minimized; maximized = $Maximized
    monitor_handle_decimal = [long]$Monitor.monitor_handle_decimal; monitor_device_name = [string]$Monitor.device_name; effective_dpi = [int]$Monitor.effective_dpi
    outer_physical_bounds = [pscustomobject]@{ left = $left; top = $top; right = $right; bottom = $bottom; width = $Width; height = $Height }
    client_physical_bounds = [pscustomobject]@{ left = $left; top = $top; right = $right; bottom = $bottom; width = $Width; height = $Height }
    client_logical_size = [pscustomobject]@{ width = [double]$Width * 96 / $Monitor.effective_dpi; height = [double]$Height * 96 / $Monitor.effective_dpi }
  }
}

function New-GoodWorld {
  $editor = New-TestMonitor $script:EditorIdentity "\\.\DISPLAY2" 101 1920 1080 96 0 0
  # The LED's current GDI name is intentionally a disposable observation
  # value.  The real topology has observed this same stable identity as
  # DISPLAY33; tests below also renumber it and require acceptance.
  $led = New-TestMonitor $script:LedIdentity "\\.\DISPLAY33" 102 1920 1080 144 1920 0
  $projector = New-TestMonitor $script:ProjectorIdentity "\\.\DISPLAY3" 103 3840 2160 144 -3840 0
  $extraA = New-TestMonitor "\\?\DISPLAY#TEST_EXTRA_A#D#{55555555-5555-5555-5555-555555555555}" "\\.\DISPLAY1" 104 1920 1080 96 0 -1080
  $extraB = New-TestMonitor "\\?\DISPLAY#TEST_EXTRA_B#E#{66666666-6666-6666-6666-666666666666}" "\\.\DISPLAY4" 105 2560 1440 96 3840 0
  [pscustomobject]@{
    monitors = @($editor, $led, $projector, $extraA, $extraB)
    candidate = [pscustomobject]@{ process_id = $script:GoodPid; native_image_path = $script:ExpectedPath }
    windows = @(
      [pscustomobject]@{ handle_decimal = [long]11; title = "Syndocal" },
      [pscustomobject]@{ handle_decimal = [long]22; title = "Syndocal Output - LED Program" },
      [pscustomobject]@{ handle_decimal = [long]33; title = "Syndocal Output - Projector Program" }
    )
    metrics = @{
      11 = (New-TestMetrics 11 "Syndocal" $script:GoodPid $editor 1920 1080)
      22 = (New-TestMetrics 22 "Syndocal Output - LED Program" $script:GoodPid $led 1920 1080)
      33 = (New-TestMetrics 33 "Syndocal Output - Projector Program" $script:GoodPid $projector 3840 2160)
    }
    output_observation = [pscustomobject]@{
      schema_version = [long]1
      source = "app-owned-read-only"
      outputs = @(
        [pscustomobject]@{ output_id = "41"; live_open = $true; label = "LED Program"; live_window_label = "video-output-41"; native_window_handle_decimal = "22" },
        [pscustomobject]@{ output_id = "42"; live_open = $true; label = "Projector Program"; live_window_label = "video-output-42"; native_window_handle_decimal = "33" }
      )
    }
  }
}

function Install-GoodWorldSeams {
  param($World)
  $script:CandidateProcessNameCalls = [System.Collections.Generic.List[string]]::new()
  $script:CdpExpectedAncestorCalls = [System.Collections.Generic.List[uint32]]::new()
  Set-TestSeam "Get-SyndocalCandidateProcesses" {
    param($ProcessName)
    [void]$script:CandidateProcessNameCalls.Add([string]$ProcessName)
    @($script:World.candidate)
  }
  Set-TestSeam "Get-ExecutableSha256" { param($Path) $script:CurrentHash }
  Set-TestSeam "Get-ExecutableByteSize" { param($Path) $script:CurrentSize }
  Set-TestSeam "Get-ExecutableProductVersion" { param($Path) $script:CurrentVersion }
  Set-TestSeam "Resolve-ThreeDisplayGitHead" { param($CheckoutRootPath) $script:CurrentHead }
  Set-TestSeam "Resolve-ThreeDisplayGitBranch" { param($CheckoutRootPath) $script:CurrentBranch }
  Set-TestSeam "Test-ThreeDisplayGitAncestor" {
    param($AncestorHead, $DescendantHead, $CheckoutRootPath)
    if (-not $script:SourceAncestorResult) {
      throw "Fail closed: artifact source HEAD '$AncestorHead' is not an ancestor of current harness HEAD '$DescendantHead'."
    }
    return $true
  }
  Set-TestSeam "Test-ThreeDisplayCheckoutClean" { param($CheckoutRootPath) $true }
  Set-TestSeam "Get-ThreeDisplayMonitorInventory" { @($script:World.monitors) }
  Set-TestSeam "Get-ThreeDisplayTopLevelWindows" { param($OwnerPid) @($script:World.windows) }
  Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" {
    param($CdpPort, $ExpectedAncestorProcessId)
    [void]$script:CdpExpectedAncestorCalls.Add([uint32]$ExpectedAncestorProcessId)
    [pscustomobject]@{
      listener_process_id = [uint32]5151
      listener_ancestor_process_ids = @([uint32]5151, [uint32]$script:GoodPid, [uint32]1)
      pages = @([pscustomobject]@{ strict_reader_succeeded = $true; command_result = $script:World.output_observation; failure = $null })
    }
  }
  Set-TestSeam "Get-ThreeDisplayWindowMetrics" { param($HandleDecimal) $script:World.metrics[[int]$HandleDecimal] }
  Set-TestSeam "Invoke-WithThreeDisplayPhysicalDpiContext" { param([scriptblock]$Action) & $Action }
  Set-TestSeam "Invoke-ThreeDisplayWindowMaximize" { param($HandleDecimal, $ExpectedProcessId, $ExpectedTitle) throw "maximize seam must not be reached by already-maximized success world" }
  Set-TestSeam "Invoke-ThreeDisplaySampleDelay" { param($Milliseconds) [void]$script:DelayCalls.Add($Milliseconds); return $Milliseconds }
}

function New-GoodConfiguration {
  param([bool]$Apply = $true)
  $script:CurrentHead = $script:GoodHead
  New-ThreeDisplayConfiguration -IsApply $Apply -AuthorityMode StandardRelease -ExecutablePath $script:ExpectedPath -Sha256 $script:GoodHash -ProductVersion $script:GoodVersion -GitHead $script:GoodHead -EditorIdentity $script:EditorIdentity -LedIdentity $script:LedIdentity -ProjectorIdentity $script:ProjectorIdentity -LedId 41 -LedLabel "LED Program" -ProjectorId 42 -ProjectorLabel "Projector Program" -CdpPort 5189 -IntervalMs 200 -Attempts 3 -CheckoutRootPath $script:CheckoutRoot -ShowAsioNodeExecutablePath ""
}

# ---- ShowAsioLocal authority-mode fixtures -------------------------------

$script:GoodCommit12 = $script:GoodShowArtifactHead.Substring(0, 12)
$script:WrongCommit12 = "deaddeaddead"
$script:ShowCheckerSha = ("ef" * 32)

function Get-TestShowArtifactDirectory {
  param([string]$Commit12 = $script:GoodCommit12)
  return Join-Path $script:CheckoutRoot ("target\show-asio-local\Syndocal_Show_ASIO_{0}_{1}_x64" -f @($script:GoodVersion, $Commit12))
}

function New-ShowArtifactFixture {
  # Real directories/files so the real derivation contract (containment,
  # parent/leaf naming, reparse ancestry, existence) executes over the
  # sandbox; hashes stay behind deterministic seams.
  param([string]$Commit12 = $script:GoodCommit12)
  $artifactDirectory = Get-TestShowArtifactDirectory -Commit12 $Commit12
  [void](New-Item -ItemType Directory -Path $artifactDirectory -Force)
  $executablePath = Join-Path $artifactDirectory "syndocal-show-asio.exe"
  if (-not (Test-Path -LiteralPath $executablePath)) {
    [IO.File]::WriteAllText($executablePath, "synthetic show-asio self-test executable", (New-Object Text.UTF8Encoding($false)))
  }
  return $executablePath
}

function New-ShowWorld {
  $world = New-GoodWorld
  $showExecutable = Join-Path (Get-TestShowArtifactDirectory) "syndocal-show-asio.exe"
  $world.candidate = [pscustomobject]@{ process_id = $script:GoodPid; native_image_path = $showExecutable }
  return $world
}

function New-ShowConfiguration {
  param(
    [bool]$Apply = $true,
    [string]$Commit12 = $script:GoodCommit12,
    [string]$ExecutablePath = ""
  )
  $script:CurrentHead = $script:GoodShowEvidenceHead
  $script:CurrentBranch = $script:GoodBranch
  $expectedShowExecutable = Join-Path (Get-TestShowArtifactDirectory -Commit12 $Commit12) "syndocal-show-asio.exe"
  $effectivePath = if ($ExecutablePath -ne "") { $ExecutablePath } else { $expectedShowExecutable }
  New-ThreeDisplayConfiguration -IsApply $Apply -AuthorityMode ShowAsioLocal -ExecutablePath $effectivePath -Sha256 $script:GoodHash -ProductVersion $script:GoodVersion -GitHead $script:GoodShowEvidenceHead -ArtifactSourceHead $script:GoodShowArtifactHead -ArtifactSourceBranch $script:GoodBranch -EditorIdentity $script:EditorIdentity -LedIdentity $script:LedIdentity -ProjectorIdentity $script:ProjectorIdentity -LedId 41 -LedLabel "LED Program" -ProjectorId 42 -ProjectorLabel "Projector Program" -CdpPort 5189 -IntervalMs 200 -Attempts 3 -CheckoutRootPath $script:CheckoutRoot -ShowAsioNodeExecutablePath ""
}

function New-PassLine {
  param([string]$ArtifactDirectory, [int]$Files = 14)
  return "Show-ASIO local artifact PASS: $ArtifactDirectory files=$Files distributionApproved=false"
}

function New-CheckerVerification {
  param(
    [int]$ExitCode = 0,
    [Parameter(Mandatory = $true)][AllowEmptyString()][string]$StandardOut,
    [string]$StandardError = "",
    [string]$ArtifactSourceHead = $script:GoodShowArtifactHead,
    [string]$EvidenceHead = $script:GoodShowEvidenceHead,
    [string]$SourceBranch = $script:GoodBranch
  )
  return [pscustomobject]@{
    exit_code = [int]$ExitCode
    standard_out = $StandardOut
    standard_error = $StandardError
    checker_path = Join-Path $script:CheckoutRoot "app\scripts\check-show-asio-artifact.mjs"
    checker_sha256 = $script:ShowCheckerSha
    node_executable_path = "C:\synthetic\node.exe"
    artifact_source_head = $ArtifactSourceHead.ToLowerInvariant()
    evidence_head = $EvidenceHead.ToLowerInvariant()
    source_branch = $SourceBranch
    checker_arguments = @("--artifact-source", $ArtifactSourceHead.ToLowerInvariant(), "--evidence-head", $EvidenceHead.ToLowerInvariant(), "--source-branch", $SourceBranch)
    invoked_at_utc = [DateTime]::UtcNow.ToString("o")
  }
}

function Install-ShowAuthoritySeams {
  # Replaces only the Node/checker process boundary and the kernel32 hard-link
  # probe; parsing, path derivation, containment, leaf naming, reparse
  # ancestry, existence, and expectation cross-checks run for real.  The
  # ShowChecker* knobs are mutable between calls so mutation-between-checks is
  # deterministically testable.
  param([Parameter(Mandatory = $true)][AllowEmptyString()][string]$StandardOut)
  $script:ShowCheckerInvocations = [System.Collections.Generic.List[object]]::new()
  Set-TestSeam "Invoke-ThreeDisplayShowAsioArtifactVerification" {
    param($CheckoutRootPath, $NodeExecutablePath, $ArtifactSourceHead, $EvidenceHead, $SourceBranch)
    [void]$script:ShowCheckerInvocations.Add([pscustomobject]@{ checkout_root = [string]$CheckoutRootPath; node = [string]$NodeExecutablePath; artifact_source_head = [string]$ArtifactSourceHead; evidence_head = [string]$EvidenceHead; source_branch = [string]$SourceBranch })
    return (New-CheckerVerification -ExitCode $script:ShowCheckerExitCode -StandardOut $script:ShowCheckerStdOut -StandardError $script:ShowCheckerStdErr -ArtifactSourceHead ([string]$ArtifactSourceHead) -EvidenceHead ([string]$EvidenceHead) -SourceBranch ([string]$SourceBranch))
  }
  Set-TestSeam "Test-ThreeDisplaySingleLinkFile" { param($Path) [bool]$script:ShowSingleLinkResult }
  $script:ShowCheckerExitCode = 0
  $script:ShowCheckerStdOut = $StandardOut
  $script:ShowCheckerStdErr = ""
  $script:ShowSingleLinkResult = $true
}

function Assert-Throws {
  param([scriptblock]$Action, [string]$Contains)
  try { & $Action | Out-Null; return [pscustomobject]@{ Passed = $false; Detail = "did not throw" } }
  catch {
    if ($_.Exception.Message.Contains($Contains)) { return [pscustomobject]@{ Passed = $true; Detail = $_.Exception.Message } }
    return [pscustomobject]@{ Passed = $false; Detail = "expected '$Contains', got '$($_.Exception.Message)'" }
  }
}

function New-Check { param([bool]$Passed, [string]$Detail) [pscustomobject]@{ Passed = $Passed; Detail = $Detail } }

function New-TestEvidenceDirectory {
  $root = Join-Path $script:SandboxRoot "evidence"
  [void](New-Item -ItemType Directory -Path $root -Force)
  return New-ThreeDisplayEvidenceDirectory -RootPath $root -Slug "selftest"
}

function Invoke-FocusedChecks {
  $saved = Save-TestSeams
  $script:SavedRunnerFunctions = $saved
  $checks = [System.Collections.Generic.List[object]]::new()
  try {
    $script:World = New-GoodWorld
    $script:CurrentHash = $script:GoodHash; $script:CurrentSize = $script:GoodSize; $script:CurrentVersion = $script:GoodVersion; $script:CurrentHead = $script:GoodHead; $script:CurrentBranch = $script:GoodBranch; $script:SourceAncestorResult = $true
    Install-GoodWorldSeams -World $script:World
    $config = New-GoodConfiguration

    $checks.Add([pscustomobject]@{ Name = "Git authority environment overrides fail closed without mutating the operator environment"; Run = {
      $overrideNames = @($script:ThreeDisplayGitAuthorityEnvironmentNames + "GIT_CONFIG_KEY_0" + "GIT_CONFIG_VALUE_0")
      $passed = $true
      $details = [System.Collections.Generic.List[string]]::new()
      foreach ($name in $overrideNames) {
        try {
          Assert-ThreeDisplayGitEnvironmentSafe -Environment @{ $name = "synthetic-override" } | Out-Null
          $passed = $false
          [void]$details.Add("did not reject $name")
        } catch {
          if (-not $_.Exception.Message.Contains($name)) {
            $passed = $false
            [void]$details.Add("wrong diagnostic for $name")
          }
        }
      }
      New-Check -Passed $passed -Detail $(if ($details.Count -eq 0) { "all Git repo/index/object/config authority overrides rejected" } else { $details -join "; " })
    } })

    $checks.Add([pscustomobject]@{ Name = "Git authority query helpers emit only their authoritative result"; Run = {
      $runnerText = [IO.File]::ReadAllText($script:RunnerPath)
      $runnerLines = [IO.File]::ReadAllLines($script:RunnerPath)
      $gitHelperNames = @("Resolve-ThreeDisplayGitHead", "Resolve-ThreeDisplayGitBranch", "Test-ThreeDisplayGitAncestor", "Test-ThreeDisplayCheckoutClean")
      $staticFailures = [System.Collections.Generic.List[string]]::new()
      foreach ($name in $gitHelperNames) {
        $start = -1
        for ($index = 0; $index -lt $runnerLines.Count; $index++) {
          if ($runnerLines[$index] -match "^function $([regex]::Escape($name)) ") { $start = $index; break }
        }
        if ($start -lt 0) {
          [void]$staticFailures.Add("helper $name is missing")
          continue
        }
        $end = $runnerLines.Count
        for ($index = $start + 1; $index -lt $runnerLines.Count; $index++) {
          if ($runnerLines[$index] -match "^function ") { $end = $index; break }
        }
        $body = ($runnerLines[$start..($end - 1)] -join "`n")
        if (([regex]::Matches($body, [regex]::Escape("[void](Assert-ThreeDisplayGitEnvironmentSafe)")).Count) -ne 1) {
          [void]$staticFailures.Add("helper $name does not suppress exactly one Git environment guard result")
        }
      }
      if (([regex]::Matches($runnerText, [regex]::Escape("[void](Assert-ThreeDisplayGitEnvironmentSafe)")).Count) -ne 4) {
        [void]$staticFailures.Add("runner does not have exactly four output-suppressed Git environment guard callsites")
      }
      if ($staticFailures.Count -ne 0) {
        return New-Check $false ($staticFailures -join "; ")
      }

      $savedResolverFunctions = @{}
      foreach ($name in $gitHelperNames) { $savedResolverFunctions[$name] = (Get-Item -LiteralPath "function:$name").ScriptBlock }
      $savedGitFunction = Get-Item -LiteralPath "function:git" -ErrorAction SilentlyContinue
      $savedLastExitCodeVariable = Get-Variable -Name LASTEXITCODE -Scope Global -ErrorAction SilentlyContinue
      $script:GitAuthorityProbeCalls = [System.Collections.Generic.List[string]]::new()
      try {
        foreach ($name in $gitHelperNames) { Set-Item -LiteralPath "function:$name" -Value $script:SavedRunnerFunctions[$name] }
        Set-Item -LiteralPath "function:git" -Value {
          $gitArgs = @($args | ForEach-Object { [string]$_ })
          [void]$script:GitAuthorityProbeCalls.Add(($gitArgs -join " "))
          $global:LASTEXITCODE = 0
          $command = if ($gitArgs.Count -gt 3) { [string]$gitArgs[3] } else { "" }
          switch ($command) {
            "rev-parse" { Write-Output $script:GoodHead }
            "branch" { Write-Output $script:GoodBranch }
            default { }
          }
        }
        $headResult = @(Resolve-ThreeDisplayGitHead -CheckoutRootPath $script:CheckoutRoot)
        $branchResult = @(Resolve-ThreeDisplayGitBranch -CheckoutRootPath $script:CheckoutRoot)
        $ancestorResult = @(Test-ThreeDisplayGitAncestor -AncestorHead $script:GoodArtifactHead -DescendantHead $script:GoodHead -CheckoutRootPath $script:CheckoutRoot)
        $cleanResult = @(Test-ThreeDisplayCheckoutClean -CheckoutRootPath $script:CheckoutRoot)
        $passed =
          ($headResult.Count -eq 1) -and ([string]$headResult[0] -ceq $script:GoodHead.ToLowerInvariant()) -and
          ($branchResult.Count -eq 1) -and ([string]$branchResult[0] -ceq $script:GoodBranch) -and
          ($ancestorResult.Count -eq 1) -and ([bool]$ancestorResult[0]) -and
          ($cleanResult.Count -eq 1) -and ([bool]$cleanResult[0]) -and
          ($script:GitAuthorityProbeCalls.Count -eq 4)
        New-Check -Passed $passed -Detail "head, branch, ancestor, and clean Git queries return one uncontaminated authoritative result"
      } finally {
        if ($null -eq $savedLastExitCodeVariable) { Remove-Variable -Name LASTEXITCODE -Scope Global -ErrorAction SilentlyContinue } else { Set-Variable -Name LASTEXITCODE -Scope Global -Value $savedLastExitCodeVariable.Value }
        foreach ($name in $gitHelperNames) { Set-Item -LiteralPath "function:$name" -Value $savedResolverFunctions[$name] }
        if ($null -eq $savedGitFunction) { Remove-Item -LiteralPath "function:git" -ErrorAction SilentlyContinue } else { Set-Item -LiteralPath "function:git" -Value $savedGitFunction.ScriptBlock }
      }
    } })

    $checks.Add([pscustomobject]@{ Name = "alpha.60 authority binds exact artifact metadata"; Run = {
      $authority = $config.artifact_authority
      $passed =
        ([string]$authority.product_version -ceq $script:GoodVersion) -and
        ([uint64]$authority.byte_size -eq $script:GoodSize) -and
        ([string]$authority.sha256 -ceq $script:GoodHash) -and
        ([string]$authority.source_branch -ceq "codex/syndocal-v1.2") -and
        ([string]$authority.source_head -ceq $script:GoodArtifactHead) -and
        ([uint64]$config.expected_byte_size -eq $script:GoodSize) -and
        ([string]$config.artifact_source_head -ceq $script:GoodArtifactHead) -and
        ([string]$config.artifact_source_branch -ceq $script:GoodBranch) -and
        ([string]$config.expected_git_head -ceq $script:GoodHead) -and
        ([string]$config.expected_git_head -ceq [string]$config.artifact_source_head)
      New-Check -Passed $passed -Detail "StandardRelease binds artifact metadata to its exact current harness source HEAD"
    } })
    $checks.Add([pscustomobject]@{ Name = "five-display inventory proves same-resolution roles require exact identities"; Run = {
      $sample = Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true
      New-Check -Passed ((@($sample.monitors).Count -eq 5 -and $sample.windows.Count -eq 3)) -Detail "five monitors and three role windows proved"
    } })
    $checks.Add([pscustomobject]@{ Name = "resolution-only identity omission is rejected"; Run = {
      $role = $config.roles[0]; $prior = $role.stable_identity; $role.stable_identity = ""; try { Assert-Throws { Get-ThreeDisplayExpectedMonitor -Role $role -Inventory @($script:World.monitors) } "resolution-only" } finally { $role.stable_identity = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "exact stable role identity acceptance records a nonblank current GDI name"; Run = {
      $sample = Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true
      $ledMonitor = @($sample.monitors | Where-Object { $_.stable_identity -ceq $script:LedIdentity })[0]
      $ledWindow = @($sample.windows | Where-Object { $_.role -ceq "led" })[0]
      $passed =
        (-not [string]::IsNullOrWhiteSpace([string]$ledMonitor.device_name)) -and
        (-not [string]::IsNullOrWhiteSpace([string]$ledWindow.window.monitor_device_name)) -and
        ([string]$ledWindow.window.monitor_device_name -ceq [string]$ledMonitor.device_name) -and
        ([int]$ledMonitor.effective_dpi -eq 144) -and
        ([int]$ledMonitor.physical_bounds.width -eq 1920) -and
        ([int]$ledMonitor.physical_bounds.height -eq 1080)
      New-Check -Passed $passed -Detail "LED selected by explicit stable identity; current nonblank GDI name was recorded without a role-hardcoded number"
    } })
    $checks.Add([pscustomobject]@{ Name = "inventory blank/whitespace current GDI name is rejected"; Run = {
      $results = foreach ($candidate in @("", "   ")) {
        Assert-Throws { Assert-ThreeDisplayNonblankCurrentGdiName -Name $candidate -FailureMessage "current GDI device name is blank for an enumerated monitor." } "current GDI device name is blank for an enumerated monitor."
      }
      $passed = (@($results | Where-Object { -not $_.Passed }).Count -eq 0)
      New-Check -Passed $passed -Detail "inventory callback guard rejects both empty and whitespace-only current GDI names"
    } })
    $checks.Add([pscustomobject]@{ Name = "selected monitor blank/whitespace device_name is rejected"; Run = {
      $prior = $script:World.monitors[1].device_name
      try {
        $results = foreach ($candidate in @("", "   ")) {
          $script:World.monitors[1].device_name = $candidate
          Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "stable identity resolved without a nonblank current GDI device name"
        }
        $passed = (@($results | Where-Object { -not $_.Passed }).Count -eq 0)
        New-Check -Passed $passed -Detail "selected stable monitor rejects both empty and whitespace-only device_name values"
      } finally { $script:World.monitors[1].device_name = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "window metrics blank/whitespace monitor_device_name is rejected"; Run = {
      $prior = $script:World.metrics[22]
      try {
        $results = foreach ($candidate in @("", "   ")) {
          $changed = New-TestMetrics 22 "Syndocal Output - LED Program" $script:GoodPid $script:World.monitors[1] 1920 1080
          $changed.monitor_device_name = $candidate
          $script:World.metrics[22] = $changed
          Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "has no nonblank current GDI device name"
        }
        $passed = (@($results | Where-Object { -not $_.Passed }).Count -eq 0)
        New-Check -Passed $passed -Detail "window metrics reject both empty and whitespace-only monitor_device_name values"
      } finally { $script:World.metrics[22] = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "role monitor collision is rejected"; Run = {
      Assert-Throws { New-ThreeDisplayConfiguration -IsApply $true -ExecutablePath $script:ExpectedPath -Sha256 $script:GoodHash -ProductVersion $script:GoodVersion -GitHead $script:GoodHead -EditorIdentity $script:EditorIdentity -LedIdentity $script:EditorIdentity -ProjectorIdentity $script:ProjectorIdentity -LedId 41 -LedLabel "LED Program" -ProjectorId 42 -ProjectorLabel "Projector Program" -CdpPort 5189 -IntervalMs 200 -Attempts 3 -CheckoutRootPath $script:CheckoutRoot } "three distinct"
    } })
    $checks.Add([pscustomobject]@{ Name = "missing monitor identity is rejected"; Run = {
      $prior = $script:World.monitors; $script:World.monitors = @($prior | Where-Object { $_.stable_identity -ne $script:ProjectorIdentity }); try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "matched 0" } finally { $script:World.monitors = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "stale monitor identity is rejected"; Run = {
      $stale = New-GoodConfiguration; $stale.roles[2].stable_identity = $script:StaleIdentity; Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $stale -RequireEditorMaximized $true } "matched 0"
    } })
    $checks.Add([pscustomobject]@{ Name = "duplicate stable monitor identity is rejected"; Run = {
      $prior = $script:World.monitors; $duplicate = New-TestMonitor $script:LedIdentity "\\.\DISPLAY9" 109 1920 1080 144 5760 0; $script:World.monitors = @($prior + $duplicate); try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "duplicate stable identities" } finally { $script:World.monitors = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "wrong native output label/title is rejected"; Run = {
      $prior = $script:World.windows; $script:World.windows = @($prior | ForEach-Object { if ($_.handle_decimal -eq 22) { [pscustomobject]@{ handle_decimal = [long]22; title = "Syndocal Output - Wrong LED" } } else { $_ } }); try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "exact native label/title" } finally { $script:World.windows = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "post-census fresh metrics title replacement is rejected before app observation"; Run = {
      $prior = $script:World.metrics[22]; $script:World.metrics[22] = New-TestMetrics 22 "Syndocal Output - Replaced LED" $script:GoodPid $script:World.monitors[1] 1920 1080; try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "fresh metrics title" } finally { $script:World.metrics[22] = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "post-census fresh metrics HWND replacement is rejected before app observation"; Run = {
      $prior = $script:World.metrics[22]; $script:World.metrics[22] = New-TestMetrics 222 "Syndocal Output - LED Program" $script:GoodPid $script:World.monitors[1] 1920 1080; try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "fresh metrics HWND" } finally { $script:World.metrics[22] = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "wrong child owner PID is rejected"; Run = {
      $prior = $script:World.metrics[22]; $script:World.metrics[22] = New-TestMetrics 22 "Syndocal Output - LED Program" $script:WrongPid $script:World.monitors[1] 1920 1080; try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "owner PID" } finally { $script:World.metrics[22] = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "wrong checkout executable path is rejected"; Run = {
      $prior = $script:World.candidate; $script:World.candidate = [pscustomobject]@{ process_id = $script:GoodPid; native_image_path = "C:\\foreign\\syndocal.exe" }; try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "exact checkout process" } finally { $script:World.candidate = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "wrong executable hash is rejected"; Run = {
      $prior = $script:CurrentHash; $script:CurrentHash = $script:WrongHash; try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "SHA256 mismatch" } finally { $script:CurrentHash = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "wrong executable byte size is rejected"; Run = {
      $prior = $script:CurrentSize; $script:CurrentSize = $script:WrongSize; try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "byte size mismatch" } finally { $script:CurrentSize = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "wrong product version is rejected"; Run = {
      $prior = $script:CurrentVersion; $script:CurrentVersion = $script:WrongVersion; try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "ProductVersion mismatch" } finally { $script:CurrentVersion = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "wrong checkout HEAD is rejected"; Run = {
      $prior = $script:CurrentHead; $script:CurrentHead = $script:WrongHead; try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "git HEAD mismatch" } finally { $script:CurrentHead = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "dirty checkout is rejected before display acceptance"; Run = {
      Set-TestSeam "Test-ThreeDisplayCheckoutClean" { param($CheckoutRootPath) throw "Fail closed: exact $($script:GoodVersion) artifact acceptance requires a clean checkout; git status reported 1 change(s)." }
      try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "requires a clean checkout" } finally { Set-TestSeam "Test-ThreeDisplayCheckoutClean" { param($CheckoutRootPath) $true } }
    } })
    $checks.Add([pscustomobject]@{ Name = "historical alpha.41 configuration is rejected"; Run = {
      Assert-Throws { New-ThreeDisplayConfiguration -IsApply $true -ExecutablePath $script:ExpectedPath -Sha256 $script:GoodHash -ProductVersion $script:StaleVersion -GitHead $script:GoodHead -EditorIdentity $script:EditorIdentity -LedIdentity $script:LedIdentity -ProjectorIdentity $script:ProjectorIdentity -LedId 41 -LedLabel "LED Program" -ProjectorId 42 -ProjectorLabel "Projector Program" -CdpPort 5189 -IntervalMs 200 -Attempts 3 -CheckoutRootPath $script:CheckoutRoot } "exactly 1.2.0-alpha.60"
    } })
    $checks.Add([pscustomobject]@{ Name = "wrong alpha.60 hash authority is rejected"; Run = {
      Assert-Throws { New-ThreeDisplayConfiguration -IsApply $true -ExecutablePath $script:ExpectedPath -Sha256 $script:WrongHash -ProductVersion $script:GoodVersion -GitHead $script:GoodHead -EditorIdentity $script:EditorIdentity -LedIdentity $script:LedIdentity -ProjectorIdentity $script:ProjectorIdentity -LedId 41 -LedLabel "LED Program" -ProjectorId 42 -ProjectorLabel "Projector Program" -CdpPort 5189 -IntervalMs 200 -Attempts 3 -CheckoutRootPath $script:CheckoutRoot } "exact 1.2.0-alpha.60 artifact authority"
    } })
    $checks.Add([pscustomobject]@{ Name = "current harness HEAD may equal alpha.60 artifact source HEAD"; Run = {
      $passed = ([string]$config.expected_git_head -ceq $script:GoodHead) -and ([string]$config.artifact_source_head -ceq $script:GoodArtifactHead) -and ([string]$config.expected_git_head -ceq [string]$config.artifact_source_head)
      New-Check -Passed $passed -Detail "current harness HEAD may equal the exact alpha.60 StandardRelease artifact source HEAD"
    } })
    $checks.Add([pscustomobject]@{ Name = "current clean descendant harness HEAD is accepted with separate artifact source authority"; Run = {
      $priorCurrentHead = $script:CurrentHead
      try {
        $descendantConfig = New-GoodConfiguration
        $descendantConfig.expected_git_head = $script:GoodDescendantHead
        $script:CurrentHead = $script:GoodDescendantHead
        $sample = Get-ThreeDisplayStrictSample -Configuration $descendantConfig -RequireEditorMaximized $true
        $passed =
          ([string]$sample.process.current_harness_head -ceq $script:GoodDescendantHead) -and
          ([string]$sample.process.artifact_source_head -ceq $script:GoodArtifactHead) -and
          ([string]$sample.process.current_harness_head -cne [string]$sample.process.artifact_source_head) -and
          ([string]$descendantConfig.expected_git_head -ceq $script:GoodDescendantHead) -and
          ([string]$descendantConfig.artifact_source_head -ceq $script:GoodArtifactHead)
        New-Check -Passed $passed -Detail "clean descendant current harness HEAD accepted while artifact_source_head remains separately bound"
      } finally { $script:CurrentHead = $priorCurrentHead }
    } })
    $checks.Add([pscustomobject]@{ Name = "current harness branch mismatch is rejected"; Run = {
      $prior = $script:CurrentBranch; $script:CurrentBranch = $script:WrongBranch
      try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "differs from required StandardRelease source branch" } finally { $script:CurrentBranch = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "alpha.60 artifact source HEAD not ancestor is rejected"; Run = {
      $prior = $script:SourceAncestorResult; $script:SourceAncestorResult = $false
      try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "not an ancestor" } finally { $script:SourceAncestorResult = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "GDI renumbering of the same stable identity is accepted"; Run = {
      $priorName = $script:World.monitors[1].device_name
      $priorMetrics = $script:World.metrics[22]
      $renumberedName = "\\.\DISPLAY5"
      $script:World.monitors[1].device_name = $renumberedName
      $script:World.metrics[22] = New-TestMetrics 22 "Syndocal Output - LED Program" $script:GoodPid $script:World.monitors[1] 1920 1080
      try {
        $sample = Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true
        $ledMonitor = @($sample.monitors | Where-Object { $_.stable_identity -ceq $script:LedIdentity })[0]
        $ledWindow = @($sample.windows | Where-Object { $_.role -ceq "led" })[0]
        $passed =
          ([string]$ledMonitor.stable_identity -ceq $script:LedIdentity) -and
          ([string]$ledMonitor.device_name -ceq $renumberedName) -and
          ([string]$ledWindow.window.monitor_device_name -ceq $renumberedName) -and
          ([int]$ledMonitor.effective_dpi -eq 144) -and
          ([int]$ledMonitor.physical_bounds.width -eq 1920) -and
          ([int]$ledMonitor.physical_bounds.height -eq 1080)
        New-Check -Passed $passed -Detail "same stable LED identity accepted after current GDI renumbering; DPI, physical resolution, and nonblank GDI evidence remained exact"
      } finally {
        $script:World.monitors[1].device_name = $priorName
        $script:World.metrics[22] = $priorMetrics
      }
    } })
    $checks.Add([pscustomobject]@{ Name = "stable-identity role swap is rejected"; Run = {
      $swapped = New-GoodConfiguration
      $swapped.roles[1].stable_identity = $script:ProjectorIdentity
      $swapped.roles[2].stable_identity = $script:LedIdentity
      Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $swapped -RequireEditorMaximized $true } "native physical resolution"
    } })
    $checks.Add([pscustomobject]@{ Name = "projector effective DPI 144 is required"; Run = {
      $prior = $script:World.monitors[2].effective_dpi; $script:World.monitors[2].effective_dpi = 96; try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "effective DPI is 96, expected 144" } finally { $script:World.monitors[2].effective_dpi = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "output HWND DPI must equal its bound monitor DPI"; Run = {
      $prior = $script:World.metrics[33]; $changed = New-TestMetrics 33 "Syndocal Output - Projector Program" $script:GoodPid $script:World.monitors[2] 3840 2160; $changed.effective_dpi = 96; $script:World.metrics[33] = $changed; try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "HWND 33 effective DPI" } finally { $script:World.metrics[33] = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "live output client must exactly fill physical monitor bounds"; Run = {
      $prior = $script:World.metrics[22]; $changed = New-TestMetrics 22 "Syndocal Output - LED Program" $script:GoodPid $script:World.monitors[1] 1920 1080; $changed.client_physical_bounds.left++; $changed.client_physical_bounds.right++; $script:World.metrics[22] = $changed; try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "do not exactly fill" } finally { $script:World.metrics[22] = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "CDP ancestry stops at the exact checkout PID before reading its parent"; Run = {
      $script:AncestryReads = [System.Collections.Generic.List[uint32]]::new()
      Set-TestSeam "Get-ThreeDisplayProcessRecord" {
        param($ProcessId)
        [void]$script:AncestryReads.Add([uint32]$ProcessId)
        if ([uint32]$ProcessId -eq [uint32]5151) { return [pscustomobject]@{ ParentProcessId = [uint32]$script:GoodPid } }
        throw "unexpected process-parent read for PID $ProcessId"
      }
      try {
        $ancestors = @(Get-ThreeDisplayProcessAncestorIds -ProcessId ([uint32]5151) -ExpectedAncestorProcessId $script:GoodPid)
        $passed =
          ($ancestors.Count -eq 2) -and
          ($ancestors[0] -eq [uint32]5151) -and
          ($ancestors[1] -eq $script:GoodPid) -and
          ($script:AncestryReads.Count -eq 1) -and
          ($script:AncestryReads[0] -eq [uint32]5151)
        New-Check -Passed $passed -Detail "expected checkout PID is included and returned before its stale parent can be queried"
      } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "CDP ancestry disappearance before the exact checkout PID fails closed"; Run = {
      $script:AncestryReads = [System.Collections.Generic.List[uint32]]::new()
      Set-TestSeam "Get-ThreeDisplayProcessRecord" {
        param($ProcessId)
        [void]$script:AncestryReads.Add([uint32]$ProcessId)
        if ([uint32]$ProcessId -eq [uint32]5151) { return [pscustomobject]@{ ParentProcessId = [uint32]5152 } }
        return $null
      }
      try {
        Assert-Throws { Get-ThreeDisplayProcessAncestorIds -ProcessId ([uint32]5151) -ExpectedAncestorProcessId $script:GoodPid } "5152 disappeared"
      } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "CDP ancestry wrong expected PID and zero terminus fail closed"; Run = {
      Set-TestSeam "Get-ThreeDisplayProcessRecord" {
        param($ProcessId)
        if ([uint32]$ProcessId -eq [uint32]5151) { return [pscustomobject]@{ ParentProcessId = [uint32]0 } }
        throw "unexpected process-parent read for PID $ProcessId"
      }
      try {
        Assert-Throws { Get-ThreeDisplayProcessAncestorIds -ProcessId ([uint32]5151) -ExpectedAncestorProcessId $script:GoodPid } "terminated before exact checkout PID"
      } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "CDP ancestry cycle before the exact checkout PID fails closed"; Run = {
      Set-TestSeam "Get-ThreeDisplayProcessRecord" {
        param($ProcessId)
        [pscustomobject]@{ ParentProcessId = [uint32]5151 }
      }
      try {
        Assert-Throws { Get-ThreeDisplayProcessAncestorIds -ProcessId ([uint32]5151) -ExpectedAncestorProcessId $script:GoodPid } "invalid or cyclic"
      } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "CDP WebSocket completion values do not leak into the observation result"; Run = {
      $script:FakeCdpSocket = [pscustomobject]@{
        response = ([ordered]@{
          id = 1
          result = [ordered]@{ result = [ordered]@{ value = [ordered]@{ hello = "world" } } }
        } | ConvertTo-Json -Depth 8 -Compress)
      }
      $script:FakeCdpSocket | Add-Member ScriptMethod ConnectAsync { param($uri, $token) [Threading.Tasks.Task]::CompletedTask }
      $script:FakeCdpSocket | Add-Member ScriptMethod SendAsync { param($segment, $messageType, $endOfMessage, $token) [Threading.Tasks.Task]::CompletedTask }
      $script:FakeCdpSocket | Add-Member ScriptMethod ReceiveAsync {
        param($segment, $token)
        $bytes = [Text.Encoding]::UTF8.GetBytes([string]$this.response)
        [Array]::Copy($bytes, 0, $segment.Array, $segment.Offset, $bytes.Length)
        return [Threading.Tasks.Task[object]]::FromResult([pscustomobject]@{
          MessageType = [System.Net.WebSockets.WebSocketMessageType]::Text
          Count = $bytes.Length
          EndOfMessage = $true
        })
      }
      $script:FakeCdpSocket | Add-Member ScriptMethod Dispose { }
      Set-TestSeam "New-ThreeDisplayCdpClientWebSocket" { return $script:FakeCdpSocket }
      try {
        $values = @(Invoke-ThreeDisplayCdpRuntimeEvaluate -WebSocketDebuggerUrl "ws://127.0.0.1:5189/devtools/page/fake" -Expression "1")
        $taskValues = @($values | Where-Object { $_ -is [Threading.Tasks.Task] })
        $passed = ($values.Count -eq 1) -and ($taskValues.Count -eq 0) -and ([string]$values[0].hello -ceq "world")
        New-Check -Passed $passed -Detail "ConnectAsync and SendAsync completion values are suppressed while the real response remains one typed result"
      } finally {
        Remove-Variable FakeCdpSocket -Scope Script -ErrorAction SilentlyContinue
        Install-GoodWorldSeams -World $script:World
      }
    } })
    $checks.Add([pscustomobject]@{ Name = "app-owned self-verified main frontend reader succeeds through the complete transport seam"; Run = {
      $observation = Get-ThreeDisplayExactOutputWindowObservation -Configuration $config
      New-Check -Passed ($observation.schema_version -eq 1 -and $observation.source -eq "app-owned-read-only" -and $observation.outputs[0].output_id -ceq "41" -and $observation.outputs[0].native_window_handle_decimal -ceq "22" -and $script:CdpExpectedAncestorCalls.Count -eq 1 -and $script:CdpExpectedAncestorCalls[0] -eq $script:GoodPid) -Detail "strict canonical string observation returned from one self-verified main reader with the exact checkout PID bound into transport"
    } })
    $checks.Add([pscustomobject]@{ Name = "ambiguous self-verified main frontend readers are rejected"; Run = {
      Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" { param($CdpPort, $ExpectedAncestorProcessId) [pscustomobject]@{ listener_process_id = [uint32]5151; listener_ancestor_process_ids = @([uint32]5151, [uint32]$script:GoodPid, [uint32]1); pages = @([pscustomobject]@{ strict_reader_succeeded = $true; command_result = $script:World.output_observation; failure = $null }, [pscustomobject]@{ strict_reader_succeeded = $true; command_result = $script:World.output_observation; failure = $null }) } }
      try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "exactly one" } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "only rejected non-main frontend readers fail closed"; Run = {
      Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" { param($CdpPort, $ExpectedAncestorProcessId) [pscustomobject]@{ listener_process_id = [uint32]5151; listener_ancestor_process_ids = @([uint32]5151, [uint32]$script:GoodPid, [uint32]1); pages = @([pscustomobject]@{ strict_reader_succeeded = $false; command_result = $null; failure = "reader is restricted to the main Tauri window" }) } }
      try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "exposed 0" } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "missing WebView page is rejected"; Run = {
      Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" { param($CdpPort, $ExpectedAncestorProcessId) [pscustomobject]@{ listener_process_id = [uint32]5151; listener_ancestor_process_ids = @([uint32]5151, [uint32]$script:GoodPid, [uint32]1); pages = @() } }
      try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "exposed 0" } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "missing strict frontend observation reader result is rejected"; Run = {
      Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" { param($CdpPort, $ExpectedAncestorProcessId) [pscustomobject]@{ listener_process_id = [uint32]5151; listener_ancestor_process_ids = @([uint32]5151, [uint32]$script:GoodPid, [uint32]1); pages = @([pscustomobject]@{ strict_reader_succeeded = $true; command_result = $null; failure = $null }) } }
      try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "returned no output observation result" } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "non-Boolean strict reader success flag is rejected"; Run = {
      Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" { param($CdpPort, $ExpectedAncestorProcessId) [pscustomobject]@{ listener_process_id = [uint32]5151; listener_ancestor_process_ids = @([uint32]5151, [uint32]$script:GoodPid, [uint32]1); pages = @([pscustomobject]@{ strict_reader_succeeded = 1; command_result = $script:World.output_observation; failure = $null }) } }
      try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "not Boolean" } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "rejected strict reader carrying a result is rejected"; Run = {
      Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" { param($CdpPort, $ExpectedAncestorProcessId) [pscustomobject]@{ listener_process_id = [uint32]5151; listener_ancestor_process_ids = @([uint32]5151, [uint32]$script:GoodPid, [uint32]1); pages = @([pscustomobject]@{ strict_reader_succeeded = $false; command_result = $script:World.output_observation; failure = "reader is restricted to the main Tauri window" }) } }
      try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "also contains a command result" } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "wrong CDP listener ancestry PID is rejected"; Run = {
      Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" { param($CdpPort, $ExpectedAncestorProcessId) [pscustomobject]@{ listener_process_id = [uint32]5151; listener_ancestor_process_ids = @([uint32]5151, [uint32]4244, [uint32]1); pages = @([pscustomobject]@{ strict_reader_succeeded = $true; command_result = $script:World.output_observation; failure = $null }) } }
      try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "not descended" } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "CDP ancestry must begin with the exact listener PID"; Run = {
      Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" { param($CdpPort, $ExpectedAncestorProcessId) [pscustomobject]@{ listener_process_id = [uint32]5151; listener_ancestor_process_ids = @([uint32]5152, [uint32]$script:GoodPid, [uint32]1); pages = @([pscustomobject]@{ strict_reader_succeeded = $true; command_result = $script:World.output_observation; failure = $null }) } }
      try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "does not begin with listener PID" } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "repeated CDP ancestry PID is rejected"; Run = {
      Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" { param($CdpPort, $ExpectedAncestorProcessId) [pscustomobject]@{ listener_process_id = [uint32]5151; listener_ancestor_process_ids = @([uint32]5151, [uint32]$script:GoodPid, [uint32]5151); pages = @([pscustomobject]@{ strict_reader_succeeded = $true; command_result = $script:World.output_observation; failure = $null }) } }
      try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "contains a repeated PID" } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "zero CDP listener PID is rejected"; Run = {
      Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" { param($CdpPort, $ExpectedAncestorProcessId) [pscustomobject]@{ listener_process_id = [uint32]0; listener_ancestor_process_ids = @([uint32]0, [uint32]$script:GoodPid); pages = @([pscustomobject]@{ strict_reader_succeeded = $true; command_result = $script:World.output_observation; failure = $null }) } }
      try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "must be one nonzero UInt32" } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "missing CDP pages collection is rejected"; Run = {
      Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" { param($CdpPort, $ExpectedAncestorProcessId) [pscustomobject]@{ listener_process_id = [uint32]5151; listener_ancestor_process_ids = @([uint32]5151, [uint32]$script:GoodPid, [uint32]1) } }
      try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "pages are missing or not an array" } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "invalid app-owned observation schema is rejected"; Run = {
      $prior = $script:World.output_observation.schema_version; $script:World.output_observation.schema_version = 2; try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "schema_version 1" } finally { $script:World.output_observation.schema_version = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "output live window label must match its exact output ID"; Run = {
      $prior = $script:World.output_observation.outputs[0].live_window_label; $script:World.output_observation.outputs[0].live_window_label = "video-output-42"; try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "must equal 'video-output-41'" } finally { $script:World.output_observation.outputs[0].live_window_label = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "hostile app-observed labels are sanitized before exception interpolation"; Run = {
      $priorLabel = $script:World.output_observation.outputs[0].label
      $priorLiveWindowLabel = $script:World.output_observation.outputs[0].live_window_label
      $hostileLabel = "bad" + [string][char]0x001F + ("x" * 500)
      $hostileLiveWindowLabel = "video-output-41" + [string][char]0x0000 + ("y" * 500)
      try {
        $script:World.output_observation.outputs[0].label = $hostileLabel
        $labelError = $null
        try { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true | Out-Null } catch { $labelError = $_.Exception.Message }
        $script:World.output_observation.outputs[0].label = $priorLabel
        $script:World.output_observation.outputs[0].live_window_label = $hostileLiveWindowLabel
        $liveLabelError = $null
        try { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config | Out-Null } catch { $liveLabelError = $_.Exception.Message }
        $expectedLabelEcho = ConvertTo-ThreeDisplayOneLineDiagnostic -Value $hostileLabel
        $expectedLiveLabelEcho = ConvertTo-ThreeDisplayOneLineDiagnostic -Value $hostileLiveWindowLabel
        $passed = ($null -ne $labelError) -and ($null -ne $liveLabelError)
        $passed = $passed -and ($labelError -notmatch "[\x00-\x1F\x7F\u0085\u2028\u2029]") -and ($liveLabelError -notmatch "[\x00-\x1F\x7F\u0085\u2028\u2029]")
        $passed = $passed -and ($labelError.Contains($expectedLabelEcho)) -and ($liveLabelError.Contains($expectedLiveLabelEcho))
        $passed = $passed -and (-not ($labelError.Contains(("x" * 401)))) -and (-not ($liveLabelError.Contains(("y" * 401))))
        New-Check -Passed $passed -Detail "hostile label and live_window_label diagnostics are Unicode-safe one-line echoes capped before exception interpolation"
      } finally {
        $script:World.output_observation.outputs[0].label = $priorLabel
        $script:World.output_observation.outputs[0].live_window_label = $priorLiveWindowLabel
      }
    } })
    $checks.Add([pscustomobject]@{ Name = "leading-zero output decimal is rejected"; Run = {
      $prior = $script:World.output_observation.outputs[0].output_id; $script:World.output_observation.outputs[0].output_id = "041"; try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "canonical positive decimal" } finally { $script:World.output_observation.outputs[0].output_id = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "overflow HWND decimal is rejected"; Run = {
      $prior = $script:World.output_observation.outputs[0].native_window_handle_decimal; $script:World.output_observation.outputs[0].native_window_handle_decimal = "18446744073709551616"; try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "unsigned 64-bit" } finally { $script:World.output_observation.outputs[0].native_window_handle_decimal = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "output ID-to-HWND mismatch is rejected"; Run = {
      $prior = $script:World.output_observation.outputs[0].native_window_handle_decimal; $script:World.output_observation.outputs[0].native_window_handle_decimal = "999"; try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "differs from the native title-selected HWND" } finally { $script:World.output_observation.outputs[0].native_window_handle_decimal = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "non-app-owned command result is rejected"; Run = {
      $prior = $script:World.output_observation.source; $script:World.output_observation.source = "operator-authored"; try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "must be schema_version 1 from the app-owned-read-only provider" } finally { $script:World.output_observation.source = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "configured rejected dry-run returns failure rather than success"; Run = {
      $dryConfig = New-GoodConfiguration -Apply $false
      $evidence = New-TestEvidenceDirectory
      Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" { param($CdpPort, $ExpectedAncestorProcessId) throw "Fail closed: loopback CDP endpoint is unavailable." }
      try {
        $result = Invoke-ThreeDisplayAcceptance -Configuration $dryConfig -EvidenceDirectory $evidence
        New-Check -Passed ((-not $result.succeeded) -and $result.verdict -eq "dry-run-rejected") -Detail "configured observation transport gap is a nonzero final-show route"
      } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "partial discovery dry-run remains explicitly not-configured"; Run = {
      $discovery = New-ThreeDisplayConfiguration -IsApply $false -ExecutablePath "" -Sha256 "" -ProductVersion "" -GitHead "" -EditorIdentity "" -LedIdentity "" -ProjectorIdentity "" -LedId 0 -LedLabel "" -ProjectorId 0 -ProjectorLabel "" -CdpPort 0 -IntervalMs 200 -Attempts 3 -CheckoutRootPath $script:CheckoutRoot
      $result = Invoke-ThreeDisplayAcceptance -Configuration $discovery -EvidenceDirectory (New-TestEvidenceDirectory)
      New-Check -Passed ($result.succeeded -and $result.verdict -eq "not-configured") -Detail "unconfigured inventory is discovery only, never acceptance"
    } })
    $checks.Add([pscustomobject]@{ Name = "wrong child monitor placement is rejected"; Run = {
      $prior = $script:World.metrics[22]; $script:World.metrics[22] = New-TestMetrics 22 "Syndocal Output - LED Program" $script:GoodPid $script:World.monitors[2] 1920 1080; try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "swap/default/first-monitor" } finally { $script:World.metrics[22] = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "minimized output is rejected"; Run = {
      $prior = $script:World.metrics[33]; $script:World.metrics[33] = New-TestMetrics 33 "Syndocal Output - Projector Program" $script:GoodPid $script:World.monitors[2] 3840 2160 $true $true $true; try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "minimized" } finally { $script:World.metrics[33] = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "hung output is rejected"; Run = {
      $prior = $script:World.metrics[33]; $script:World.metrics[33] = New-TestMetrics 33 "Syndocal Output - Projector Program" $script:GoodPid $script:World.monitors[2] 3840 2160 $true $false $false; try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "hung/unresponsive" } finally { $script:World.metrics[33] = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "three consecutive stable samples pass without UI mutation"; Run = {
      $script:DelayCalls.Clear(); $result = Invoke-ThreeDisplayApplyAcceptance -Configuration $config; New-Check -Passed ($result.accepted -and $result.samples.Count -eq 3 -and $script:DelayCalls.Count -eq 2 -and -not $result.operation.performed) -Detail "three stable samples and two deterministic delay seams"
    } })
    $checks.Add([pscustomobject]@{ Name = "editor maximize receives exact freshly revalidated HWND identity"; Run = {
      $prior = $script:World.metrics[11]
      $script:World.metrics[11] = New-TestMetrics 11 "Syndocal" $script:GoodPid $script:World.monitors[0] 1920 1080 $false
      $script:MaximizeObservation = $null
      Set-TestSeam "Invoke-ThreeDisplayWindowMaximize" {
        param($HandleDecimal, $ExpectedProcessId, $ExpectedTitle)
        $script:MaximizeObservation = [pscustomobject]@{ handle = $HandleDecimal; process_id = $ExpectedProcessId; title = $ExpectedTitle }
        $script:World.metrics[11].maximized = $true
        return $true
      }
      try {
        $result = Invoke-ThreeDisplayApplyAcceptance -Configuration $config
        New-Check -Passed ($result.accepted -and $result.operation.performed -and $script:MaximizeObservation.handle -eq 11 -and $script:MaximizeObservation.process_id -eq $script:GoodPid -and $script:MaximizeObservation.title -ceq "Syndocal") -Detail "only exact revalidated editor identity reached the maximize seam"
      } finally {
        $script:World.metrics[11] = $prior
        Install-GoodWorldSeams -World $script:World
      }
    } })
    $checks.Add([pscustomobject]@{ Name = "real maximize body issues exactly SW_MAXIMIZE through narrow native seams"; Run = {
      Install-NativeMaximizeSeams
      try {
        $ok = Invoke-ThreeDisplayWindowMaximize -HandleDecimal ([long]11) -ExpectedProcessId $script:GoodPid -ExpectedTitle "Syndocal"
        $call = $script:NativeWorld.showcalls[0]
        New-Check -Passed ([bool]$ok -and $script:NativeWorld.showcalls.Count -eq 1 -and $call.handle -eq [long]11 -and $call.command -eq 3 -and $call.command -eq $script:ThreeDisplaySwMaximize) -Detail "one ShowWindow call on HWND 11 with the exact SW_MAXIMIZE command 3"
      } finally { Install-GoodWorldSeams -World $script:World; Restore-NativeMaximizeSeams }
    } })
    $checks.Add([pscustomobject]@{ Name = "real maximize body fails closed when IsWindow reports a dead HWND"; Run = {
      Install-NativeMaximizeSeams -Alive $false
      try { $r = Assert-Throws { Invoke-ThreeDisplayWindowMaximize -HandleDecimal ([long]11) -ExpectedProcessId $script:GoodPid -ExpectedTitle "Syndocal" } "disappeared before maximize"; New-Check -Passed ($r.Passed -and $script:NativeWorld.showcalls.Count -eq 0) -Detail $r.Detail } finally { Install-GoodWorldSeams -World $script:World; Restore-NativeMaximizeSeams }
    } })
    $checks.Add([pscustomobject]@{ Name = "real maximize body fails closed on changed owner PID"; Run = {
      Install-NativeMaximizeSeams -OwnerPid $script:WrongPid
      try { $r = Assert-Throws { Invoke-ThreeDisplayWindowMaximize -HandleDecimal ([long]11) -ExpectedProcessId $script:GoodPid -ExpectedTitle "Syndocal" } "owner PID changed before maximize"; New-Check -Passed ($r.Passed -and $script:NativeWorld.showcalls.Count -eq 0) -Detail $r.Detail } finally { Install-GoodWorldSeams -World $script:World; Restore-NativeMaximizeSeams }
    } })
    $checks.Add([pscustomobject]@{ Name = "real maximize body fails closed on changed window title"; Run = {
      Install-NativeMaximizeSeams -Title "Syndocal "
      try { $r = Assert-Throws { Invoke-ThreeDisplayWindowMaximize -HandleDecimal ([long]11) -ExpectedProcessId $script:GoodPid -ExpectedTitle "Syndocal" } "title changed before maximize"; New-Check -Passed ($r.Passed -and $script:NativeWorld.showcalls.Count -eq 0) -Detail $r.Detail } finally { Install-GoodWorldSeams -World $script:World; Restore-NativeMaximizeSeams }
    } })
    $checks.Add([pscustomobject]@{ Name = "real maximize body fails closed when ShowWindow reports failure"; Run = {
      Install-NativeMaximizeSeams -ShowResult $false
      try {
        $thrown = Assert-Throws { Invoke-ThreeDisplayWindowMaximize -HandleDecimal ([long]11) -ExpectedProcessId $script:GoodPid -ExpectedTitle "Syndocal" } "ShowWindow(SW_MAXIMIZE) failed"
        $issued = ($script:NativeWorld.showcalls.Count -eq 1 -and $script:NativeWorld.showcalls[0].command -eq 3)
        New-Check -Passed ($thrown.Passed -and $issued) -Detail "failure thrown after exactly one SW_MAXIMIZE attempt"
      } finally { Install-GoodWorldSeams -World $script:World; Restore-NativeMaximizeSeams }
    } })
    $checks.Add([pscustomobject]@{ Name = "centralized diagnostic sanitizer is single-line, capped, and surrogate-safe"; Run = {
      $capped = ConvertTo-ThreeDisplayOneLineDiagnostic -Value (("a" * 399) + [string][char]0xD83D + [string][char]0xDE00 + "tail")
      $collapsed = ConvertTo-ThreeDisplayOneLineDiagnostic -Value ("x`ny" + [string][char]0x2028 + "z`t")
      $last = if ($capped.Length -gt 0) { $capped[$capped.Length - 1] } else { [char]" " }
      New-Check -Passed (
        ($capped.Length -le 400) -and ($capped -ceq ("a" * 399)) -and -not [char]::IsHighSurrogate($last) -and
        ($collapsed -ceq "x y z")
      ) -Detail "cap 400 drops a straddling surrogate pair whole and collapses every line break to one space"
    } })
    $checks.Add([pscustomobject]@{ Name = "centralized diagnostic sanitizer uses its fallback over empty results"; Run = {
      $out = ConvertTo-ThreeDisplayOneLineDiagnostic -Value ("`r`n" + [string][char]0x2028 + [string][char]0x0009) -Fallback "fallback-diagnostic"
      New-Check -Passed ($out -ceq "fallback-diagnostic") -Detail "whitespace-only diagnostics resolve to the explicit fallback"
    } })
    $checks.Add([pscustomobject]@{ Name = "default evidence root resolves to the gitignored checkout-local target\\qa tree"; Run = {
      $checkoutRoot = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
      $defaultRoot = Get-ThreeDisplayDefaultEvidenceRoot
      $expectedRoot = [IO.Path]::GetFullPath((Join-Path $checkoutRoot "target\qa"))
      $gitignore = @(Get-Content -LiteralPath (Join-Path $checkoutRoot ".gitignore") -ErrorAction Stop | ForEach-Object { $_.Trim() })
      $ignoresTarget = @($gitignore | Where-Object { $_ -match "^/?target/?$" }).Count -gt 0
      New-Check -Passed (
        ($defaultRoot -ceq $expectedRoot) -and
        $defaultRoot.StartsWith($checkoutRoot, [StringComparison]::OrdinalIgnoreCase) -and
        ($defaultRoot.EndsWith("target\qa", [StringComparison]::OrdinalIgnoreCase)) -and
        $ignoresTarget
      ) -Detail "default evidence root '$defaultRoot' equals checkout target\\qa and .gitignore excludes /target/"
    } })
    # ---- ShowAsioLocal authority-mode hostile and success checks ----------

    $checks.Add([pscustomobject]@{ Name = "unknown Show-ASIO authority mode value is rejected"; Run = {
      $r = Assert-Throws { New-ThreeDisplayConfiguration -IsApply $true -AuthorityMode "ShowAsioGeneric" -ExecutablePath "" -Sha256 $script:GoodHash -ProductVersion $script:GoodVersion -GitHead $script:GoodHead -EditorIdentity $script:EditorIdentity -LedIdentity $script:LedIdentity -ProjectorIdentity $script:ProjectorIdentity -LedId 41 -LedLabel "LED Program" -ProjectorId 42 -ProjectorLabel "Projector Program" -CdpPort 5189 -IntervalMs 200 -Attempts 3 -CheckoutRootPath $script:CheckoutRoot -ShowAsioNodeExecutablePath "" } "AuthorityMode"
      New-Check -Passed ($r.Passed) -Detail $r.Detail
    } })
    $checks.Add([pscustomobject]@{ Name = "StandardRelease Apply generic alternate exe remains rejected"; Run = {
      $r = Assert-Throws { New-ThreeDisplayConfiguration -IsApply $true -AuthorityMode StandardRelease -ExecutablePath "C:\foreign\syndocal.exe" -Sha256 $script:GoodHash -ProductVersion $script:GoodVersion -GitHead $script:GoodHead -EditorIdentity $script:EditorIdentity -LedIdentity $script:LedIdentity -ProjectorIdentity $script:ProjectorIdentity -LedId 41 -LedLabel "LED Program" -ProjectorId 42 -ProjectorLabel "Projector Program" -CdpPort 5189 -IntervalMs 200 -Attempts 3 -CheckoutRootPath $script:CheckoutRoot -ShowAsioNodeExecutablePath "" } "target\\release\\syndocal.exe"
      New-Check -Passed ($r.Passed) -Detail $r.Detail
    } })
    $checks.Add([pscustomobject]@{ Name = "ShowAsioLocal caller path outside the show tree is rejected"; Run = {
      $r = Assert-Throws { New-ShowConfiguration -ExecutablePath (Join-Path $script:CheckoutRoot "target\release\syndocal.exe") } "generic alternate executables"
      New-Check -Passed ($r.Passed) -Detail $r.Detail
    } })
    $checks.Add([pscustomobject]@{ Name = "ShowAsioLocal caller path with wrong executable leaf is rejected"; Run = {
      $foreignLeaf = Join-Path (Get-TestShowArtifactDirectory) "other-tool.exe"
      $r = Assert-Throws { New-ShowConfiguration -ExecutablePath $foreignLeaf } "must end in exactly syndocal-show-asio.exe"
      New-Check -Passed ($r.Passed) -Detail $r.Detail
    } })
    $checks.Add([pscustomobject]@{ Name = "ShowAsioLocal does not inherit StandardRelease alpha.60 product lock"; Run = {
      $showAlpha14 = New-ThreeDisplayConfiguration -IsApply $false -AuthorityMode ShowAsioLocal -ExecutablePath "" -Sha256 $script:GoodHash -ProductVersion "1.2.0-alpha.14" -GitHead $script:GoodShowEvidenceHead -ArtifactSourceHead $script:GoodShowArtifactHead -ArtifactSourceBranch $script:GoodBranch -EditorIdentity $script:EditorIdentity -LedIdentity $script:LedIdentity -ProjectorIdentity $script:ProjectorIdentity -LedId 41 -LedLabel "LED Program" -ProjectorId 42 -ProjectorLabel "Projector Program" -CdpPort 5189 -IntervalMs 200 -Attempts 3 -CheckoutRootPath $script:CheckoutRoot -ShowAsioNodeExecutablePath ""
      $passed = ([string]$showAlpha14.expected_product_version -ceq "1.2.0-alpha.14") -and ($null -eq $showAlpha14.artifact_authority) -and ([uint64]$showAlpha14.expected_byte_size -eq 0)
      New-Check -Passed $passed -Detail "ShowAsioLocal retains checker/manifest-bound product/version authority; StandardRelease alpha.60 metadata is not applied"
    } })
    $checks.Add([pscustomobject]@{ Name = "nonzero Show-ASIO checker exit fails closed before any derivation"; Run = {
      [void](New-ShowArtifactFixture)
      Install-ShowAuthoritySeams -StandardOut (New-PassLine (Get-TestShowArtifactDirectory))
      $script:ShowCheckerExitCode = 1
      try {
        $config = New-ShowConfiguration
        $r = Assert-Throws { Invoke-ThreeDisplayShowAsioAuthorityGate -Configuration $config -Phase "pre-mutation" } "exited with '1'"
        New-Check -Passed ($r.Passed -and $script:ShowCheckerInvocations.Count -eq 1) -Detail $r.Detail
      } finally { Install-ShowAuthoritySeams -StandardOut (New-PassLine (Get-TestShowArtifactDirectory)) | Out-Null }
    } })
    $checks.Add([pscustomobject]@{ Name = "fake checker stdout shapes are rejected by the exact PASS parser"; Run = {
      $validLine = New-PassLine (Get-TestShowArtifactDirectory)
      $hostileLines = @(
        @{ name = "multi-line stdout"; stdout = "noise$([Environment]::NewLine)$validLine"; stderr = ""; exit = 0; expect = "not exactly one line" },
        @{ name = "wrong prefix"; stdout = "PASS: $(Get-TestShowArtifactDirectory) files=14 distributionApproved=false"; stderr = ""; exit = 0; expect = "not the exact single-line PASS contract" },
        @{ name = "future approval true"; stdout = "$(Get-TestShowArtifactDirectory) files=14 distributionApproved=true"; stderr = ""; exit = 0; expect = "not the exact single-line PASS contract" },
        @{ name = "missing files clause"; stdout = "Show-ASIO local artifact PASS: $(Get-TestShowArtifactDirectory) distributionApproved=false"; stderr = ""; exit = 0; expect = "not the exact single-line PASS contract" },
        @{ name = "zero files"; stdout = "Show-ASIO local artifact PASS: $(Get-TestShowArtifactDirectory) files=0 distributionApproved=false"; stderr = ""; exit = 0; expect = "not the exact single-line PASS contract" },
        @{ name = "empty stdout"; stdout = ""; stderr = ""; exit = 0; expect = "not exactly one line" },
        @{ name = "padded line"; stdout = " $validLine"; stderr = ""; exit = 0; expect = "leading or trailing whitespace" },
        @{ name = "stderr noise"; stdout = $validLine; stderr = "[show-asio-check] hint"; exit = 0; expect = "unexpected stderr" },
        @{ name = "relative directory"; stdout = "Show-ASIO local artifact PASS: target\show-asio-local\Syndocal_Show_ASIO_$($script:GoodVersion)_facefaceface_x64 files=3 distributionApproved=false"; stderr = ""; exit = 0; expect = "plain rooted Windows path" }
      )
      $allRejected = $true
      foreach ($hostile in $hostileLines) {
        $r = Assert-Throws { ConvertTo-ThreeDisplayShowAsioVerifiedPassContract -Verification (New-CheckerVerification -ExitCode $hostile.exit -StandardOut $hostile.stdout -StandardError $hostile.stderr) } $hostile.expect
        if (-not $r.Passed) { $allRejected = $false; break }
      }
      New-Check -Passed $allRejected -Detail "nine hostile stdout/stderr shapes failed closed at '$($hostile.expect)'"
    } })
     $checks.Add([pscustomobject]@{ Name = "exact PASS parser succeeds and derives only the attested directory"; Run = {
       $parsed = ConvertTo-ThreeDisplayShowAsioVerifiedPassContract -Verification (New-CheckerVerification -StandardOut (New-PassLine (Get-TestShowArtifactDirectory) -Files 14))
       New-Check -Passed (($parsed.files_verified -eq 14) -and ($parsed.artifact_directory_raw -ceq (Get-TestShowArtifactDirectory))) -Detail "one exact PASS contract parsed"
     } })
     $checks.Add([pscustomobject]@{ Name = "ShowAsioLocal authority rejects checker S/E/B drift"; Run = {
       [void](New-ShowArtifactFixture)
       Install-ShowAuthoritySeams -StandardOut (New-PassLine (Get-TestShowArtifactDirectory))
       try {
         $config = New-ShowConfiguration
         Set-TestSeam "Invoke-ThreeDisplayShowAsioArtifactVerification" {
           param($CheckoutRootPath, $NodeExecutablePath, $ArtifactSourceHead, $EvidenceHead, $SourceBranch)
           return (New-CheckerVerification -StandardOut $script:ShowCheckerStdOut -ArtifactSourceHead $script:GoodHead -EvidenceHead $EvidenceHead -SourceBranch $SourceBranch)
         }
         $r = Assert-Throws { Invoke-ThreeDisplayShowAsioAuthorityGate -Configuration $config -Phase "pre-mutation" } "artifact source S differs"
         New-Check -Passed $r.Passed -Detail $r.Detail
       } finally { Install-ShowAuthoritySeams -StandardOut (New-PassLine (Get-TestShowArtifactDirectory)) | Out-Null }
     } })
     $checks.Add([pscustomobject]@{ Name = "checker-derived directory outside this checkout is rejected"; Run = {
      [void](New-ShowArtifactFixture)
      Install-ShowAuthoritySeams -StandardOut (New-PassLine "C:\foreign-checkout\target\show-asio-local\Syndocal_Show_ASIO_$($script:GoodVersion)_facefaceface_x64")
      try {
        $config = New-ShowConfiguration
        $r = Assert-Throws { Invoke-ThreeDisplayShowAsioAuthorityGate -Configuration $config -Phase "pre-mutation" } "outside this checkout"
        New-Check -Passed ($r.Passed) -Detail $r.Detail
      } finally { Install-ShowAuthoritySeams -StandardOut (New-PassLine (Get-TestShowArtifactDirectory)) | Out-Null }
    } })
    $checks.Add([pscustomobject]@{ Name = "artifact directory leaf commit12 mismatch is rejected"; Run = {
      $driftedExe = New-ShowArtifactFixture -Commit12 $script:WrongCommit12
      Install-ShowAuthoritySeams -StandardOut (New-PassLine (Split-Path -Parent $driftedExe))
      try {
        $config = New-ShowConfiguration
         $r = Assert-Throws { Invoke-ThreeDisplayShowAsioAuthorityGate -Configuration $config -Phase "pre-mutation" } "binds source commit '$($script:WrongCommit12)'"
        New-Check -Passed ($r.Passed) -Detail $r.Detail
      } finally { Install-ShowAuthoritySeams -StandardOut (New-PassLine (Get-TestShowArtifactDirectory)) | Out-Null }
    } })
    $checks.Add([pscustomobject]@{ Name = "artifact directory leaf version drift is rejected"; Run = {
      $driftedDirectory = Join-Path $script:CheckoutRoot "target\show-asio-local\Syndocal_Show_ASIO_9.9.9_$($script:GoodCommit12)_x64"
      [void](New-Item -ItemType Directory -Path $driftedDirectory -Force)
      [void](New-ShowArtifactFixture -Commit12 $script:GoodCommit12)
      Copy-Item -LiteralPath (Join-Path (Get-TestShowArtifactDirectory) "syndocal-show-asio.exe") -Destination (Join-Path $driftedDirectory "syndocal-show-asio.exe") -Force
      Install-ShowAuthoritySeams -StandardOut (New-PassLine $driftedDirectory)
      try {
        $config = New-ShowConfiguration
        $r = Assert-Throws { Invoke-ThreeDisplayShowAsioAuthorityGate -Configuration $config -Phase "pre-mutation" } "binds version '9.9.9'"
        New-Check -Passed ($r.Passed) -Detail $r.Detail
      } finally { Install-ShowAuthoritySeams -StandardOut (New-PassLine (Get-TestShowArtifactDirectory)) | Out-Null }
    } })
    $checks.Add([pscustomobject]@{ Name = "caller executable expectation differing from the derived artifact is rejected"; Run = {
      # Caller pins the wrong-commit artifact directory (a fully valid shape),
      # while the verified checker result derives the HEAD-matching directory;
      # only the explicit caller cross-check may reject this.
      [void](New-ShowArtifactFixture -Commit12 $script:WrongCommit12)
      [void](New-ShowArtifactFixture -Commit12 $script:GoodCommit12)
      Install-ShowAuthoritySeams -StandardOut (New-PassLine (Get-TestShowArtifactDirectory -Commit12 $script:GoodCommit12))
      try {
        $config = New-ShowConfiguration -Commit12 $script:WrongCommit12
        $r = Assert-Throws { Invoke-ThreeDisplayShowAsioAuthorityGate -Configuration $config -Phase "pre-mutation" } "caller ExpectedExecutablePath"
        New-Check -Passed ($r.Passed) -Detail $r.Detail
      } finally { Install-ShowAuthoritySeams -StandardOut (New-PassLine (Get-TestShowArtifactDirectory)) | Out-Null }
    } })
    $checks.Add([pscustomobject]@{ Name = "mutation between the two authority checks fails closed"; Run = {
      [void](New-ShowArtifactFixture)
      Install-ShowAuthoritySeams -StandardOut (New-PassLine (Get-TestShowArtifactDirectory))
      try {
        $config = New-ShowConfiguration
        $first = Invoke-ThreeDisplayShowAsioAuthorityGate -Configuration $config -Phase "pre-mutation"
        $prior = $script:CurrentHash
        $script:CurrentHash = $script:WrongHash
        try {
          $r = Assert-Throws { Invoke-ThreeDisplayShowAsioAuthorityGate -Configuration $config -Phase "pre-executable-use" } "SHA-256 mismatch between checks or versus caller expectation"
          New-Check -Passed ($r.Passed -and ([string]$first.phase -ceq "pre-mutation") -and $script:ShowCheckerInvocations.Count -eq 2) -Detail $r.Detail
        } finally { $script:CurrentHash = $prior }
      } finally { Install-ShowAuthoritySeams -StandardOut (New-PassLine (Get-TestShowArtifactDirectory)) | Out-Null }
    } })
    $checks.Add([pscustomobject]@{ Name = "hard-linked artifact executable is rejected"; Run = {
      [void](New-ShowArtifactFixture)
      Install-ShowAuthoritySeams -StandardOut (New-PassLine (Get-TestShowArtifactDirectory))
      $script:ShowSingleLinkResult = $false
      try {
        $config = New-ShowConfiguration
        $r = Assert-Throws { Invoke-ThreeDisplayShowAsioAuthorityGate -Configuration $config -Phase "pre-mutation" } "hard link"
        New-Check -Passed ($r.Passed) -Detail $r.Detail
      } finally { Install-ShowAuthoritySeams -StandardOut (New-PassLine (Get-TestShowArtifactDirectory)) | Out-Null }
    } })
    $checks.Add([pscustomobject]@{ Name = "reparse-backed artifact directory ancestry is rejected"; Run = {
      # A separate sandbox checkout keeps every earlier leaf/commit expectation
      # valid so derivation reaches the real reparse-ancestry probe.
      $alternateCheckout = Join-Path $script:SandboxRoot "show-checkout-reparse"
      $junctionDirectory = Join-Path $alternateCheckout (Join-Path "target\show-asio-local" ("Syndocal_Show_ASIO_{0}_{1}_x64" -f @($script:GoodVersion, $script:GoodCommit12)))
      $junctionTarget = Join-Path $script:SandboxRoot "show-junction-target"
      [void](New-Item -ItemType Directory -Path $junctionTarget -Force)
      $createdJunction = $false
      try {
        [void](New-Item -ItemType Junction -Path $junctionDirectory -Target $junctionTarget -ErrorAction Stop)
        $createdJunction = $true
        Install-ShowAuthoritySeams -StandardOut (New-PassLine $junctionDirectory)
        $config = New-ThreeDisplayConfiguration -IsApply $true -AuthorityMode ShowAsioLocal -ExecutablePath (Join-Path $junctionDirectory "syndocal-show-asio.exe") -Sha256 $script:GoodHash -ProductVersion $script:GoodVersion -GitHead $script:GoodShowEvidenceHead -ArtifactSourceHead $script:GoodShowArtifactHead -ArtifactSourceBranch $script:GoodBranch -EditorIdentity $script:EditorIdentity -LedIdentity $script:LedIdentity -ProjectorIdentity $script:ProjectorIdentity -LedId 41 -LedLabel "LED Program" -ProjectorId 42 -ProjectorLabel "Projector Program" -CdpPort 5189 -IntervalMs 200 -Attempts 3 -CheckoutRootPath $alternateCheckout -ShowAsioNodeExecutablePath ""
        $r = Assert-Throws { Invoke-ThreeDisplayShowAsioAuthorityGate -Configuration $config -Phase "pre-mutation" } "reparse point"
        New-Check -Passed ($r.Passed) -Detail $r.Detail
      } catch {
        if (-not $createdJunction) { return New-Check $false "could not create a junction to prove rejection: $($_.Exception.Message)" }
        throw
      } finally {
        Install-ShowAuthoritySeams -StandardOut (New-PassLine (Get-TestShowArtifactDirectory)) | Out-Null
        if ($createdJunction -and (Test-Path -LiteralPath $junctionDirectory)) { [void][IO.Directory]::Delete($junctionDirectory, $false) }
      }
    } })
    $checks.Add([pscustomobject]@{ Name = "real kernel32 hard-link probe proves one link for the fixture executable"; Run = {
      $showExecutable = New-ShowArtifactFixture
      $realProbe = [scriptblock]::Create($script:SavedRunnerFunctions["Test-ThreeDisplaySingleLinkFile"].ToString())
      New-Check -Passed ([bool](& $realProbe -Path $showExecutable)) -Detail "GetFileInformationByHandle reports NumberOfLinks=1 for the synthetic artifact executable"
    } })
    $checks.Add([pscustomobject]@{ Name = "ShowAsioLocal Apply succeeds only through the exact manifest-validated path"; Run = {
      $showExecutable = New-ShowArtifactFixture
      $script:World = New-ShowWorld
      Install-GoodWorldSeams -World $script:World
      Install-ShowAuthoritySeams -StandardOut (New-PassLine (Get-TestShowArtifactDirectory))
      try {
        $config = New-ShowConfiguration
        $result = Invoke-ThreeDisplayApplyAcceptance -Configuration $config
        $verifications = @($result.authority_verifications)
        $passed =
          ([bool]$result.accepted) -and ($verifications.Count -eq 2) -and
          ((@($verifications | ForEach-Object { $_.phase }) -join ",") -ceq "pre-mutation,pre-executable-use") -and
          ($script:ShowCheckerInvocations.Count -eq 2) -and
          (@($script:CandidateProcessNameCalls | Where-Object { $_ -cne "syndocal-show-asio" })).Count -eq 0 -and          ([string]$verifications[0].executable_path -ieq $showExecutable) -and
          ([string]$verifications[1].executable_sha256 -ceq $script:GoodHash) -and
          ([uint64]$verifications[1].executable_byte_size -eq $script:GoodSize) -and
          ([string]$verifications[1].executable_product_version -ceq $script:GoodVersion) -and
           ([string]$verifications[1].checkout_git_head -ceq $script:GoodShowEvidenceHead) -and
           ([string]$verifications[1].artifact_source_head -ceq $script:GoodShowArtifactHead) -and
           ([string]$verifications[1].artifact_source_branch -ceq $script:GoodBranch) -and
           ([string]$verifications[1].evidence_head -ceq $script:GoodShowEvidenceHead) -and
           ((@($verifications[1].checker_arguments) -join "|") -ceq ("--artifact-source|{0}|--evidence-head|{1}|--source-branch|{2}" -f @($script:GoodShowArtifactHead, $script:GoodShowEvidenceHead, $script:GoodBranch))) -and
          ([string]$verifications[1].checker_sha256 -ceq $script:ShowCheckerSha) -and
          ([string]$verifications[1].artifact_flavor -ceq "windows-show-asio-local-only") -and
          ($verifications[1].distribution_approved -eq $false) -and
          ([string]$verifications[1].pass_line.StartsWith("Show-ASIO local artifact PASS: ")) -and
          ([long]$verifications[1].files_verified -gt 0) -and
          (-not [string]::IsNullOrWhiteSpace([string]$verifications[1].verified_at_utc)) -and
          ([string]$config.expected_executable_path -ieq $showExecutable)
         New-Check -Passed $passed -Detail "two-phase authority verifications bound distinct artifact source S, evidence HEAD E, source branch B, flavor/checker/path/hash/byte-size/version and both times"
      } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "ShowAsioLocal Apply maximize path re-verifies immediately before the UI mutation"; Run = {
      [void](New-ShowArtifactFixture)
      $script:World = New-ShowWorld
      $priorMetrics = $script:World.metrics[11]
      $script:World.metrics[11] = New-TestMetrics 11 "Syndocal" $script:GoodPid $script:World.monitors[0] 1920 1080 $false
      Install-GoodWorldSeams -World $script:World
      Install-ShowAuthoritySeams -StandardOut (New-PassLine (Get-TestShowArtifactDirectory))
      $script:MaximizeObservation = $null
      Set-TestSeam "Invoke-ThreeDisplayWindowMaximize" {
        param($HandleDecimal, $ExpectedProcessId, $ExpectedTitle)
        $script:MaximizeObservation = [pscustomobject]@{ handle = $HandleDecimal; process_id = $ExpectedProcessId; title = $ExpectedTitle }
        $script:World.metrics[11].maximized = $true
        return $true
      }
      try {
        $config = New-ShowConfiguration
        $result = Invoke-ThreeDisplayApplyAcceptance -Configuration $config
        $verifications = @($result.authority_verifications)
        New-Check -Passed (
          ([bool]$result.accepted) -and ($verifications.Count -eq 2) -and
          ([string]$verifications[1].phase -ceq "pre-executable-use") -and
          ([bool]$result.operation.performed) -and ($script:MaximizeObservation.handle -eq 11)
        ) -Detail "second authority verification ran immediately before the only UI mutation"
      } finally {
        $script:World.metrics[11] = $priorMetrics
        Install-GoodWorldSeams -World $script:World
      }
    } })
    $checks.Add([pscustomobject]@{ Name = "ShowAsioLocal strict sample refuses a bypassed or unresolved executable path"; Run = {
      [void](New-ShowArtifactFixture)
      $script:World = New-ShowWorld
      Install-GoodWorldSeams -World $script:World
      Install-ShowAuthoritySeams -StandardOut (New-PassLine (Get-TestShowArtifactDirectory))
      try {
        $config = New-ShowConfiguration -ExecutablePath " "
        $unresolved = Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "before the authority gate resolved"
        $config.expected_executable_path = Join-Path $script:CheckoutRoot "target\release\syndocal.exe"
        $bypassed = Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "refuses generic alternate executable"
        if (-not ($unresolved.Passed -and $bypassed.Passed)) {
          return New-Check $false "unresolved=$($unresolved.Detail) | bypassed=$($bypassed.Detail)"
        }
        New-Check -Passed $true -Detail "unresolved and bypassed paths both fail closed at sample time"
      } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "ShowAsioLocal dry-run binds one gate verification and stays read-only"; Run = {
      [void](New-ShowArtifactFixture)
      $script:World = New-ShowWorld
      Install-GoodWorldSeams -World $script:World
      Install-ShowAuthoritySeams -StandardOut (New-PassLine (Get-TestShowArtifactDirectory))
      try {
        $config = New-ShowConfiguration -Apply $false
        $evidence = New-TestEvidenceDirectory
        $result = Invoke-ThreeDisplayAcceptance -Configuration $config -EvidenceDirectory $evidence
        $provenance = (Get-Content -LiteralPath (Join-Path $evidence "provenance.json") -Raw | ConvertFrom-Json)
        $finalEvidence = (Get-Content -LiteralPath (Join-Path $evidence "final.json") -Raw | ConvertFrom-Json)
        $passed =
          ($result.succeeded) -and ([string]$result.verdict -ceq "dry-run-would-accept") -and
          (@($finalEvidence.authority_verifications).Count -eq 1) -and
          ([string]@($finalEvidence.authority_verifications)[0].phase -ceq "dry-run-pre-executable-use") -and
           ([string]$provenance.authority_mode -ceq "ShowAsioLocal") -and
           ([string]$provenance.artifact_source_head -ceq $script:GoodShowArtifactHead) -and
           ([string]$provenance.artifact_source_branch -ceq $script:GoodBranch) -and
           ([string]$provenance.evidence_head -ceq $script:GoodShowEvidenceHead) -and
           ([string]$provenance.expectations.evidence_head -ceq $script:GoodShowEvidenceHead) -and
           ([string]$provenance.show_asio_authority_contract.artifact_flavor -ceq "windows-show-asio-local-only") -and
          ($provenance.safety.launches_or_terminates_processes -eq $false)
        New-Check -Passed $passed -Detail "single dry-run authority verification bound into final.json and provenance.json"
      } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "StandardRelease samples keep querying exactly the syndocal image name"; Run = {
      $script:World = New-GoodWorld
      Install-GoodWorldSeams -World $script:World
      $sampleCountBefore = $script:CandidateProcessNameCalls.Count
      [void](Get-ThreeDisplayStrictSample -Configuration (New-GoodConfiguration) -RequireEditorMaximized $true)
      $observed = @($script:CandidateProcessNameCalls | Select-Object -Skip $sampleCountBefore)
      New-Check -Passed (($observed.Count -ge 1) -and (@($observed | Where-Object { $_ -cne "syndocal" }).Count -eq 0)) -Detail "standard release process selection is unchanged"
    } })

    $checks.Add([pscustomobject]@{ Name = "runner static contract forbids process and output mutations"; Run = {
      $text = [IO.File]::ReadAllText($script:RunnerPath)
      foreach ($token in @("Start-Process", "Stop-Process", "Remove-Item", "SetForegroundWindow", "SetWindowPos", "SendInput", "Invoke-WebRequest", "New-WebServiceProxy", 'hardware_or_network_access', 'qa\artifacts')) { if ($text.Contains($token)) { return New-Check $false "forbidden token $token" } }
      if ($text.Contains("1.2.0-alpha.41")) { return New-Check $false "historical alpha.41 authority remains in the runner" }
      foreach ($token in @("video-output-", "Syndocal Output - ", "resolution-only", "SHA256SUMS.txt", "GetDisplayConfigBufferSizes", "QueryDisplayConfig", "DisplayConfigGetDeviceInfo", "GetDpiForWindow", "get_video_output_window_observation_v1", "app-owned-read-only", "native_window_handle_decimal", "__syndocalReadVideoOutputWindowObservationV1", "strict_reader_succeeded", "Get-NetTCPConnection", "ClientWebSocket", "CdpPort", "1.2.0-alpha.60", "29045BC40227F823E0A2259113E2BECC246B665AEC161E5A14111D9CB0E4EA1C", "62435840", "dabbcaec8157b7c4f648af82516ee607394a7c90", "source_provenance", "artifact_source_head", "artifact_source_branch", "ExpectedArtifactSourceHead", "ExpectedArtifactSourceBranch", "current_harness_head", "current_harness_branch", "Resolve-ThreeDisplayGitBranch", "Test-ThreeDisplayGitAncestor", "expected_byte_size", "dry-run-rejected", "native_hardware_claim", '--artifact-source', '--evidence-head', '--source-branch', 'ConvertTo-ThreeDisplayOneLineDiagnostic', 'non_loopback_network_access', 'loopback_cdp_observation_only', 'complete five-display identity acceptance', 'SW_MAXIMIZE', 'Join-Path $script:ThreeDisplayCheckoutRoot "target\qa"', 'stable_identity = [string]$target.MonitorDevicePath', 'Assert-ThreeDisplayNonblankCurrentGdiName', 'current GDI device name', 'expected_effective_dpi', 'physical_bounds',
        "StandardRelease", "ShowAsioLocal", "check-show-asio-artifact.mjs", "syndocal-show-asio.exe", "windows-show-asio-local-only", "target\show-asio-local", "Syndocal_Show_ASIO_", "Show-ASIO local artifact PASS: ", "distributionApproved=false", "show-asio-local-manifest.json", "NumberOfLinks", "pre-executable-use", "pre-mutation", "dry-run-pre-executable-use", "authority_verifications", "show_asio_authority_contract", "invoked_at_utc")) { if (-not $text.Contains($token)) { return New-Check $false "required token $token missing" } }
       foreach ($token in @("Assert-ThreeDisplayGitEnvironmentSafe", "--no-replace-objects", "ThreeDisplayShowAsioSourceIdentityCount = 70", 'app\src\uiLocalization.ts')) { if (-not $text.Contains($token)) { return New-Check $false "required source/Git authority token $token missing" } }
       if (([regex]::Matches($text, [regex]::Escape("--no-replace-objects")).Count) -ne 4) { return New-Check $false "all four harness Git authority calls must disable refs/replace object substitution" }
       $transport = (Get-Command Get-ThreeDisplayCdpTransportObservation).ScriptBlock.ToString()
       $runtimeEvaluate = (Get-Command Invoke-ThreeDisplayCdpRuntimeEvaluate).ScriptBlock.ToString()
       $completionLines = @($runtimeEvaluate -split "`r?`n" | Where-Object { $_ -match '\$socket\.(ConnectAsync|SendAsync).*GetResult\(\)' })
       $suppressedCompletionLines = @($completionLines | Where-Object { $_ -match '^\s*\[void\]\$socket\.(ConnectAsync|SendAsync).*GetResult\(\)\s*$' })
       if ($completionLines.Count -ne 2 -or $suppressedCompletionLines.Count -ne 2) { return New-Check $false "all CDP ConnectAsync/SendAsync completion values must be explicitly suppressed" }
       if ($transport.Contains("api.invoke('get_video_output_window_observation_v1')")) { return New-Check $false "transport bypasses the strict frontend observation reader" }
      foreach ($retired in @("plugin:window|get_current_window", "__TAURI_INTERNALS__", "expected_gdi_device_name", "\\.\DISPLAY2", "\\.\DISPLAY3", "\\.\DISPLAY5")) { if ($text.Contains($retired)) { return New-Check $false "retired or hardcoded GDI role authority $retired remains" } }
      New-Check $true "static safety and identity contract present"
    } })

    foreach ($definition in $checks) {
      try { $outcome = & $definition.Run; $passed = [bool]$outcome.Passed; $detail = [string]$outcome.Detail }
      catch { $passed = $false; $detail = "UNEXPECTED: $($_.Exception.Message) [$($_.InvocationInfo.PositionMessage.Trim())]" }
      $definition | Add-Member -NotePropertyName Passed -NotePropertyValue $passed
      $definition | Add-Member -NotePropertyName Detail -NotePropertyValue $detail
    }
    return @($checks)
  } finally { Restore-TestSeams -Saved $saved }
}

try {
  [void](New-Item -ItemType Directory -Path (Split-Path -Parent $script:ExpectedPath) -Force)
  [IO.File]::WriteAllText($script:ExpectedPath, "synthetic self-test executable", (New-Object Text.UTF8Encoding($false)))
  $results = Invoke-FocusedChecks
  foreach ($result in $results) {
    $status = if ($result.Passed) { "PASS" } else { "FAIL" }
    "$status $($result.Name) -- $($result.Detail)"
  }
  $failed = @($results | Where-Object { -not $_.Passed }).Count
  "SUMMARY: $($results.Count) checks, $failed failed"
  if ($failed -gt 0) { exit 1 }
} finally {
  if (Test-Path -LiteralPath $script:SandboxRoot) { [IO.Directory]::Delete($script:SandboxRoot, $true) }
}
