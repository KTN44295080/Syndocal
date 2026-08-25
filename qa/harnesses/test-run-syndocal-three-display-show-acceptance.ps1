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
$script:GoodVersion = "1.2.0-alpha.12"
$script:WrongVersion = "9.9.9-wrong"
$script:GoodHead = ("face" * 10)
$script:WrongHead = ("dead" * 10)
$script:DelayCalls = [System.Collections.Generic.List[int]]::new()
$script:EditorIdentity = "\\?\DISPLAY#TEST_EDITOR#A#{11111111-1111-1111-1111-111111111111}"
$script:LedIdentity = "\\?\DISPLAY#TEST_LED#B#{22222222-2222-2222-2222-222222222222}"
$script:ProjectorIdentity = "\\?\DISPLAY#TEST_PROJECTOR#C#{33333333-3333-3333-3333-333333333333}"
$script:StaleIdentity = "\\?\DISPLAY#TEST_STALE#Z#{44444444-4444-4444-4444-444444444444}"

$script:SeamNames = @(
  "Get-SyndocalCandidateProcesses",
  "Get-ExecutableSha256",
  "Get-ExecutableProductVersion",
  "Resolve-ThreeDisplayGitHead",
  "Test-ThreeDisplayCheckoutClean",
  "Get-ThreeDisplayMonitorInventory",
  "Get-ThreeDisplayTopLevelWindows",
  "Get-ThreeDisplayCdpTransportObservation",
  "Get-ThreeDisplayWindowMetrics",
  "Invoke-WithThreeDisplayPhysicalDpiContext",
  "Invoke-ThreeDisplayWindowMaximize",
  "Invoke-ThreeDisplaySampleDelay",
  "Get-ThreeDisplayWindowTitle",
  "Test-ThreeDisplayNativeIsWindow",
  "Get-ThreeDisplayNativeOwnerProcessId",
  "Invoke-ThreeDisplayNativeShowWindow"
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
  $led = New-TestMonitor $script:LedIdentity "\\.\DISPLAY5" 102 1920 1080 144 1920 0
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
  Set-TestSeam "Get-SyndocalCandidateProcesses" { @($script:World.candidate) }
  Set-TestSeam "Get-ExecutableSha256" { param($Path) $script:CurrentHash }
  Set-TestSeam "Get-ExecutableProductVersion" { param($Path) $script:CurrentVersion }
  Set-TestSeam "Resolve-ThreeDisplayGitHead" { param($CheckoutRootPath) $script:CurrentHead }
  Set-TestSeam "Test-ThreeDisplayCheckoutClean" { param($CheckoutRootPath) $true }
  Set-TestSeam "Get-ThreeDisplayMonitorInventory" { @($script:World.monitors) }
  Set-TestSeam "Get-ThreeDisplayTopLevelWindows" { param($OwnerPid) @($script:World.windows) }
  Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" {
    param($CdpPort)
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
  New-ThreeDisplayConfiguration -IsApply $Apply -ExecutablePath $script:ExpectedPath -Sha256 $script:GoodHash -ProductVersion $script:GoodVersion -GitHead $script:GoodHead -EditorIdentity $script:EditorIdentity -LedIdentity $script:LedIdentity -ProjectorIdentity $script:ProjectorIdentity -LedId 41 -LedLabel "LED Program" -ProjectorId 42 -ProjectorLabel "Projector Program" -CdpPort 5189 -IntervalMs 200 -Attempts 3 -CheckoutRootPath $script:CheckoutRoot
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
      New-Check -Passed ($sample.windows[1].window.monitor_device_name -eq "\\.\DISPLAY5") -Detail "LED resolved by exact stable identity and DISPLAY5 role contract"
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
    $checks.Add([pscustomobject]@{ Name = "wrong product version is rejected"; Run = {
      $prior = $script:CurrentVersion; $script:CurrentVersion = $script:WrongVersion; try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "ProductVersion mismatch" } finally { $script:CurrentVersion = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "wrong checkout HEAD is rejected"; Run = {
      $prior = $script:CurrentHead; $script:CurrentHead = $script:WrongHead; try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "git HEAD mismatch" } finally { $script:CurrentHead = $prior }
    } })
    $checks.Add([pscustomobject]@{ Name = "dirty checkout is rejected before display acceptance"; Run = {
      Set-TestSeam "Test-ThreeDisplayCheckoutClean" { param($CheckoutRootPath) throw "Fail closed: exact alpha.12 artifact acceptance requires a clean checkout; git status reported 1 change(s)." }
      try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "requires a clean checkout" } finally { Set-TestSeam "Test-ThreeDisplayCheckoutClean" { param($CheckoutRootPath) $true } }
    } })
    $checks.Add([pscustomobject]@{ Name = "non-alpha.12 configuration is rejected"; Run = {
      Assert-Throws { New-ThreeDisplayConfiguration -IsApply $true -ExecutablePath $script:ExpectedPath -Sha256 $script:GoodHash -ProductVersion "1.2.0-alpha.11" -GitHead $script:GoodHead -EditorIdentity $script:EditorIdentity -LedIdentity $script:LedIdentity -ProjectorIdentity $script:ProjectorIdentity -LedId 41 -LedLabel "LED Program" -ProjectorId 42 -ProjectorLabel "Projector Program" -CdpPort 5189 -IntervalMs 200 -Attempts 3 -CheckoutRootPath $script:CheckoutRoot } "exactly 1.2.0-alpha.12"
    } })
    $checks.Add([pscustomobject]@{ Name = "wrong exact GDI role binding is rejected"; Run = {
      $prior = $script:World.monitors[1].device_name; $script:World.monitors[1].device_name = "\\.\DISPLAY4"; try { Assert-Throws { Get-ThreeDisplayStrictSample -Configuration $config -RequireEditorMaximized $true } "expected '\\.\DISPLAY5'" } finally { $script:World.monitors[1].device_name = $prior }
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
    $checks.Add([pscustomobject]@{ Name = "app-owned self-verified main frontend reader succeeds through the complete transport seam"; Run = {
      $observation = Get-ThreeDisplayExactOutputWindowObservation -Configuration $config
      New-Check -Passed ($observation.schema_version -eq 1 -and $observation.source -eq "app-owned-read-only" -and $observation.outputs[0].output_id -ceq "41" -and $observation.outputs[0].native_window_handle_decimal -ceq "22") -Detail "strict canonical string observation returned from one self-verified main reader"
    } })
    $checks.Add([pscustomobject]@{ Name = "ambiguous self-verified main frontend readers are rejected"; Run = {
      Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" { param($CdpPort) [pscustomobject]@{ listener_process_id = [uint32]5151; listener_ancestor_process_ids = @([uint32]5151, [uint32]$script:GoodPid, [uint32]1); pages = @([pscustomobject]@{ strict_reader_succeeded = $true; command_result = $script:World.output_observation; failure = $null }, [pscustomobject]@{ strict_reader_succeeded = $true; command_result = $script:World.output_observation; failure = $null }) } }
      try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "exactly one" } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "only rejected non-main frontend readers fail closed"; Run = {
      Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" { param($CdpPort) [pscustomobject]@{ listener_process_id = [uint32]5151; listener_ancestor_process_ids = @([uint32]5151, [uint32]$script:GoodPid, [uint32]1); pages = @([pscustomobject]@{ strict_reader_succeeded = $false; command_result = $null; failure = "reader is restricted to the main Tauri window" }) } }
      try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "exposed 0" } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "missing WebView page is rejected"; Run = {
      Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" { param($CdpPort) [pscustomobject]@{ listener_process_id = [uint32]5151; listener_ancestor_process_ids = @([uint32]5151, [uint32]$script:GoodPid, [uint32]1); pages = @() } }
      try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "exposed 0" } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "missing strict frontend observation reader result is rejected"; Run = {
      Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" { param($CdpPort) [pscustomobject]@{ listener_process_id = [uint32]5151; listener_ancestor_process_ids = @([uint32]5151, [uint32]$script:GoodPid, [uint32]1); pages = @([pscustomobject]@{ strict_reader_succeeded = $true; command_result = $null; failure = $null }) } }
      try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "returned no output observation result" } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "non-Boolean strict reader success flag is rejected"; Run = {
      Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" { param($CdpPort) [pscustomobject]@{ listener_process_id = [uint32]5151; listener_ancestor_process_ids = @([uint32]5151, [uint32]$script:GoodPid, [uint32]1); pages = @([pscustomobject]@{ strict_reader_succeeded = 1; command_result = $script:World.output_observation; failure = $null }) } }
      try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "not Boolean" } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "rejected strict reader carrying a result is rejected"; Run = {
      Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" { param($CdpPort) [pscustomobject]@{ listener_process_id = [uint32]5151; listener_ancestor_process_ids = @([uint32]5151, [uint32]$script:GoodPid, [uint32]1); pages = @([pscustomobject]@{ strict_reader_succeeded = $false; command_result = $script:World.output_observation; failure = "reader is restricted to the main Tauri window" }) } }
      try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "also contains a command result" } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "wrong CDP listener ancestry PID is rejected"; Run = {
      Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" { param($CdpPort) [pscustomobject]@{ listener_process_id = [uint32]5151; listener_ancestor_process_ids = @([uint32]5151, [uint32]4244, [uint32]1); pages = @([pscustomobject]@{ strict_reader_succeeded = $true; command_result = $script:World.output_observation; failure = $null }) } }
      try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "not descended" } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "CDP ancestry must begin with the exact listener PID"; Run = {
      Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" { param($CdpPort) [pscustomobject]@{ listener_process_id = [uint32]5151; listener_ancestor_process_ids = @([uint32]5152, [uint32]$script:GoodPid, [uint32]1); pages = @([pscustomobject]@{ strict_reader_succeeded = $true; command_result = $script:World.output_observation; failure = $null }) } }
      try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "does not begin with listener PID" } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "repeated CDP ancestry PID is rejected"; Run = {
      Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" { param($CdpPort) [pscustomobject]@{ listener_process_id = [uint32]5151; listener_ancestor_process_ids = @([uint32]5151, [uint32]$script:GoodPid, [uint32]5151); pages = @([pscustomobject]@{ strict_reader_succeeded = $true; command_result = $script:World.output_observation; failure = $null }) } }
      try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "contains a repeated PID" } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "zero CDP listener PID is rejected"; Run = {
      Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" { param($CdpPort) [pscustomobject]@{ listener_process_id = [uint32]0; listener_ancestor_process_ids = @([uint32]0, [uint32]$script:GoodPid); pages = @([pscustomobject]@{ strict_reader_succeeded = $true; command_result = $script:World.output_observation; failure = $null }) } }
      try { Assert-Throws { Get-ThreeDisplayExactOutputWindowObservation -Configuration $config } "must be one nonzero UInt32" } finally { Install-GoodWorldSeams -World $script:World }
    } })
    $checks.Add([pscustomobject]@{ Name = "missing CDP pages collection is rejected"; Run = {
      Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" { param($CdpPort) [pscustomobject]@{ listener_process_id = [uint32]5151; listener_ancestor_process_ids = @([uint32]5151, [uint32]$script:GoodPid, [uint32]1) } }
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
      Set-TestSeam "Get-ThreeDisplayCdpTransportObservation" { param($CdpPort) throw "Fail closed: loopback CDP endpoint is unavailable." }
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
    $checks.Add([pscustomobject]@{ Name = "runner static contract forbids process and output mutations"; Run = {
      $text = [IO.File]::ReadAllText($script:RunnerPath)
      foreach ($token in @("Start-Process", "Stop-Process", "Remove-Item", "SetForegroundWindow", "SetWindowPos", "SendInput", "Invoke-WebRequest", "New-WebServiceProxy", 'hardware_or_network_access', 'qa\artifacts')) { if ($text.Contains($token)) { return New-Check $false "forbidden token $token" } }
      foreach ($token in @("video-output-", "Syndocal Output - ", "resolution-only", "SHA256SUMS.txt", "GetDisplayConfigBufferSizes", "QueryDisplayConfig", "DisplayConfigGetDeviceInfo", "GetDpiForWindow", "get_video_output_window_observation_v1", "app-owned-read-only", "native_window_handle_decimal", "__syndocalReadVideoOutputWindowObservationV1", "strict_reader_succeeded", "Get-NetTCPConnection", "ClientWebSocket", "CdpPort", "1.2.0-alpha.12", "\\.\DISPLAY2", "\\.\DISPLAY5", "\\.\DISPLAY3", "dry-run-rejected", "native_hardware_claim", 'ConvertTo-ThreeDisplayOneLineDiagnostic', 'non_loopback_network_access', 'loopback_cdp_observation_only', 'complete five-display identity acceptance', 'SW_MAXIMIZE', 'Join-Path $script:ThreeDisplayCheckoutRoot "target\qa"')) { if (-not $text.Contains($token)) { return New-Check $false "required token $token missing" } }
      $transport = (Get-Command Get-ThreeDisplayCdpTransportObservation).ScriptBlock.ToString()
      if ($transport.Contains("api.invoke('get_video_output_window_observation_v1')")) { return New-Check $false "transport bypasses the strict frontend observation reader" }
      foreach ($retired in @("plugin:window|get_current_window", "__TAURI_INTERNALS__")) { if ($text.Contains($retired)) { return New-Check $false "retired or raw window-label path $retired remains" } }
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
