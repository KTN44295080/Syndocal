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
$script:GoodHash = ("ab" * 32)
$script:WrongHash = ("cd" * 32)
$script:GoodVersion = "1.2.0-alpha.11-selftest"
$script:WrongVersion = "9.9.9-wrong"
$script:GoodHead = ("face" * 10)
$script:WrongHead = ("dead" * 10)
$script:DelayCalls = [System.Collections.Generic.List[int]]::new()

$script:SeamNames = @(
  "Get-SyndocalCandidateProcesses",
  "Get-ExecutableSha256",
  "Get-ExecutableProductVersion",
  "Resolve-ThreeDisplayGitHead",
  "Get-ThreeDisplayMonitorInventory",
  "Get-ThreeDisplayTopLevelWindows",
  "Get-ThreeDisplayWindowMetrics",
  "Invoke-WithThreeDisplayPhysicalDpiContext",
  "Invoke-ThreeDisplayWindowMaximize",
  "Invoke-ThreeDisplaySampleDelay"
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

function New-TestMonitor {
  param([string]$Identity, [string]$Device, [long]$Handle, [int]$Width, [int]$Height, [int]$Dpi = 96)
  [pscustomobject]@{
    stable_identity = $Identity; device_name = $Device; friendly_name = $Identity
    monitor_handle_decimal = $Handle; connected = $true; effective_dpi = $Dpi
    physical_bounds = [pscustomobject]@{ left = 0; top = 0; right = $Width; bottom = $Height; width = $Width; height = $Height }
    work_area = [pscustomobject]@{ left = 0; top = 0; right = $Width; bottom = $Height; width = $Width; height = $Height }
  }
}

function New-TestMetrics {
  param([long]$Handle, [string]$Title, [uint32]$OwnerPid, $Monitor, [int]$Width, [int]$Height, [bool]$Maximized = $true, [bool]$Minimized = $false, [bool]$Responding = $true)
  [pscustomobject]@{
    handle_decimal = $Handle; title = $Title; owner_pid = $OwnerPid; alive = $true; visible = $true
    responding = $Responding; minimized = $Minimized; maximized = $Maximized
    monitor_handle_decimal = [long]$Monitor.monitor_handle_decimal; monitor_device_name = [string]$Monitor.device_name; effective_dpi = [int]$Monitor.effective_dpi
    outer_physical_bounds = [pscustomobject]@{ left = 0; top = 0; right = $Width; bottom = $Height; width = $Width; height = $Height }
    client_physical_bounds = [pscustomobject]@{ left = 0; top = 0; right = $Width; bottom = $Height; width = $Width; height = $Height }
    client_logical_size = [pscustomobject]@{ width = [double]$Width * 96 / $Monitor.effective_dpi; height = [double]$Height * 96 / $Monitor.effective_dpi }
  }
}

function New-GoodWorld {
  $editor = New-TestMonitor "DISPLAY\\EDITOR-1080\\A" "\\.\DISPLAY1" 101 1920 1080 96
  $led = New-TestMonitor "DISPLAY\\LED-1080\\B" "\\.\DISPLAY2" 102 1920 1080 120
  $projector = New-TestMonitor "DISPLAY\\PROJECTOR-4K\\C" "\\.\DISPLAY3" 103 3840 2160 144
  $extraA = New-TestMonitor "DISPLAY\\EXTRA-1080\\D" "\\.\DISPLAY4" 104 1920 1080 96
  $extraB = New-TestMonitor "DISPLAY\\EXTRA-1440\\E" "\\.\DISPLAY5" 105 2560 1440 96
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
  }
}

function Install-GoodWorldSeams {
  param($World)
  Set-TestSeam "Get-SyndocalCandidateProcesses" { @($script:World.candidate) }
  Set-TestSeam "Get-ExecutableSha256" { param($Path) $script:CurrentHash }
  Set-TestSeam "Get-ExecutableProductVersion" { param($Path) $script:CurrentVersion }
  Set-TestSeam "Resolve-ThreeDisplayGitHead" { param($CheckoutRootPath) $script:CurrentHead }
  Set-TestSeam "Get-ThreeDisplayMonitorInventory" { @($script:World.monitors) }
  Set-TestSeam "Get-ThreeDisplayTopLevelWindows" { param($OwnerPid) @($script:World.windows) }
  Set-TestSeam "Get-ThreeDisplayWindowMetrics" { param($HandleDecimal) $script:World.metrics[[int]$HandleDecimal] }
  Set-TestSeam "Invoke-WithThreeDisplayPhysicalDpiContext" { param([scriptblock]$Action) & $Action }
  Set-TestSeam "Invoke-ThreeDisplayWindowMaximize" { param($HandleDecimal) throw "maximize seam must not be reached by already-maximized success world" }
  Set-TestSeam "Invoke-ThreeDisplaySampleDelay" { param($Milliseconds) [void]$script:DelayCalls.Add($Milliseconds); return $Milliseconds }
}

function New-GoodConfiguration {
  param([bool]$Apply = $true)
  New-ThreeDisplayConfiguration -IsApply $Apply -ExecutablePath $script:ExpectedPath -Sha256 $script:GoodHash -ProductVersion $script:GoodVersion -GitHead $script:GoodHead -EditorIdentity "DISPLAY\\EDITOR-1080\\A" -LedIdentity "DISPLAY\\LED-1080\\B" -ProjectorIdentity "DISPLAY\\PROJECTOR-4K\\C" -LedId 41 -LedLabel "LED Program" -ProjectorId 42 -ProjectorLabel "Projector Program" -IntervalMs 200 -Attempts 3 -CheckoutRootPath $script:CheckoutRoot
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

function Invoke-FocusedChecks {
  $saved = Save-TestSeams
  $checks = [System.Collections.Generic.List[object]]::new()
  try {
    $script:World = New-GoodWorld
    $script:CurrentHash = $script:GoodHash; $script:CurrentVersion = $script:GoodVersion; $script:CurrentHead = $script:GoodHead
    Install-GoodWorldSeams -World $script:World
    $config = New-GoodConfiguration

    $checks.Add([pscustomobject]@{ Name = "five-display inventory proves same-resolution roles require exact identities"; Run = {
      $sample = Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true
      New-Check -Passed ((@($sample.monitors).Count -eq 5 -and $sample.windows.Count -eq 3)) -Detail "five monitors and three role windows proved"
    } })
    $checks.Add([pscustomobject]@{ Name = "resolution-only identity omission is rejected"; Run = {
      $role = $config.roles[0]; $prior = $role.stable_identity; $role.stable_identity = ""; try { Assert-Throws { Get-ThreeDisplayExpectedMonitor -Role $role -Inventory @($script:World.monitors) } "resolution-only" } finally { $role.stable_identity = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "exact role identity acceptance succeeds"; Run = {
      $sample = Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true
      New-Check -Passed ($sample.windows[1].window.monitor_device_name -eq "\\.\DISPLAY2") -Detail "LED resolved by exact stable identity"
    } })
    $checks.Add([pscustomobject]@{ Name = "role monitor collision is rejected"; Run = {
      Assert-Throws { New-ThreeDisplayConfiguration -IsApply $true -ExecutablePath $script:ExpectedPath -Sha256 $script:GoodHash -ProductVersion $script:GoodVersion -GitHead $script:GoodHead -EditorIdentity "DISPLAY\\EDITOR-1080\\A" -LedIdentity "DISPLAY\\EDITOR-1080\\A" -ProjectorIdentity "DISPLAY\\PROJECTOR-4K\\C" -LedId 41 -LedLabel "LED Program" -ProjectorId 42 -ProjectorLabel "Projector Program" -IntervalMs 200 -Attempts 3 -CheckoutRootPath $script:CheckoutRoot } "three distinct"
    } })
    $checks.Add([pscustomobject]@{ Name = "missing monitor identity is rejected"; Run = {
      $prior = $script:World.monitors; $script:World.monitors = @($prior | Where-Object { $_.stable_identity -ne "DISPLAY\\PROJECTOR-4K\\C" }); try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "matched 0" } finally { $script:World.monitors = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "stale monitor identity is rejected"; Run = {
      $stale = New-GoodConfiguration; $stale.roles[2].stable_identity = "DISPLAY\\STALE\\Z"; Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $stale -RequireEditorMaximized $true } "matched 0"
    } })
    $checks.Add([pscustomobject]@{ Name = "duplicate stable monitor identity is rejected"; Run = {
      $prior = $script:World.monitors; $duplicate = New-TestMonitor "DISPLAY\\LED-1080\\B" "\\.\DISPLAY9" 109 1920 1080; $script:World.monitors = @($prior + $duplicate); try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "duplicate stable identities" } finally { $script:World.monitors = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "wrong native output label/title is rejected"; Run = {
      $prior = $script:World.windows; $script:World.windows = @($prior | ForEach-Object { if ($_.handle_decimal -eq 22) { [pscustomobject]@{ handle_decimal = [long]22; title = "Syndocal Output - Wrong LED" } } else { $_ } }); try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "exact native label/title" } finally { $script:World.windows = $prior }
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
    $checks.Add([pscustomobject]@{ Name = "wrong product version is rejected"; Run = {
      $prior = $script:CurrentVersion; $script:CurrentVersion = $script:WrongVersion; try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "ProductVersion mismatch" } finally { $script:CurrentVersion = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "wrong checkout HEAD is rejected"; Run = {
      $prior = $script:CurrentHead; $script:CurrentHead = $script:WrongHead; try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "git HEAD mismatch" } finally { $script:CurrentHead = $prior }
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
    $checks.Add([pscustomobject]@{ Name = "runner static contract forbids process and output mutations"; Run = {
      $text = [IO.File]::ReadAllText($script:RunnerPath)
      foreach ($token in @("Start-Process", "Stop-Process", "Remove-Item", "SetForegroundWindow", "SetWindowPos", "SendInput", "Invoke-WebRequest", "New-WebServiceProxy")) { if ($text.Contains($token)) { return New-Check $false "forbidden token $token" } }
      foreach ($token in @("video-output-", "Syndocal Output - ", "resolution-only", "SHA256SUMS.txt", "GetDisplayConfigBufferSizes", "QueryDisplayConfig", "DisplayConfigGetDeviceInfo", "dry-run", "native_hardware_claim")) { if (-not $text.Contains($token)) { return New-Check $false "required token $token missing" } }
      New-Check $true "static safety and identity contract present"
    } })

    foreach ($definition in $checks) {
      try { $outcome = & $definition.Run; $passed = [bool]$outcome.Passed; $detail = [string]$outcome.Detail }
      catch { $passed = $false; $detail = "UNEXPECTED: $($_.Exception.Message)" }
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
