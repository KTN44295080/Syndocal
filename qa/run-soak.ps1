<#
.SYNOPSIS
    Runs the M5 soak harness, gated by the exact local Windows MSVC linker gate.

.DESCRIPTION
    Before any Windows Cargo build this script initializes (or requires) the exact
    VS2022 Community x64 environment: vcvars64.bat -vcvars_ver=14.44 with
    VCToolsInstallDir 14.44.35207, pins CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER
    to the absolute Hostx64/x64 link.exe, verifies with where.exe link.exe that the
    exact linker resolves first, and fails closed on Git/stale/missing/empty
    resolution. Non-Windows hosts keep the previous ungated behavior.

    -PreflightOnly runs only that gate verification and exits before any Cargo or
    soak-harness launch (on non-Windows hosts it reports that the gate does not
    apply and exits).

    Hermetic self-test seam: $global:SYNDOCAL_RUN_SOAK_TEST_DEPENDENCIES maps the
    keys "captureVcvarsEnvironment", "locateLinkers", "fileIsRegular", and
    "invokeCargo" to scriptblocks overriding the native dependencies. It is
    reserved for qa/test-run-soak-linker-gate.ps1 and must stay unset otherwise.
#>
param(
    [ValidateRange(1, 86400)]
    [int]$DurationSeconds = 3600,
    [ValidateRange(1, 60)]
    [int]$SampleIntervalSeconds = 1,
    [string]$ReportPath = "target/qa/m5-soak.json",
    [ValidateSet("Debug", "Release")]
    [string]$Configuration = "Release",
    [switch]$SkipBuild,
    [switch]$MixedLighting,
    [switch]$PreflightOnly
)

$ErrorActionPreference = "Stop"

$script:RequiredVcToolsVersion = "14.44.35207"
$script:RequiredVcToolsInstallDir = "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207"
$script:RequiredMsvcLinker = "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe"
$script:RequiredVcvarsBatch = "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
$script:RequiredVcvarsArguments = "-vcvars_ver=14.44"

function Get-SoakTestDependencyOverride {
    param([Parameter(Mandatory = $true)][string]$Name)
    $overrides = $global:SYNDOCAL_RUN_SOAK_TEST_DEPENDENCIES
    if ($null -ne $overrides -and $overrides.ContainsKey($Name)) {
        $override = $overrides[$Name]
        if ($null -ne $override) { return $override }
    }
    return $null
}

function Resolve-SoakDependency {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][scriptblock]$Default
    )
    $override = Get-SoakTestDependencyOverride -Name $Name
    if ($null -ne $override) { return $override }
    return $Default
}

function ConvertTo-PathComparisonKey {
    param([Parameter(Mandatory = $true)][AllowEmptyString()][string]$Candidate)
    return $Candidate.Replace("/", "\").Trim().ToLowerInvariant()
}

function Test-RegularFilePath {
    param([Parameter(Mandatory = $true)][string]$Path)
    return Test-Path -LiteralPath $Path -PathType Leaf
}

function Get-MsvcHostLinkerPath {
    param([Parameter(Mandatory = $true)][string]$VcToolsInstallDir)
    return Join-Path (Join-Path (Join-Path (Join-Path $VcToolsInstallDir "bin") "Hostx64") "x64") "link.exe"
}

function ConvertFrom-CommandLineSetOutput {
    param([Parameter(Mandatory = $true)][AllowEmptyString()][string]$Output)
    $parsedEnvironment = @{}
    foreach ($line in ($Output -split "\r?\n")) {
        if ($line -match "^([A-Za-z_][A-Za-z0-9_]*)=(.*)$") {
            $parsedEnvironment[$Matches[1]] = $Matches[2]
        }
    }
    if ($parsedEnvironment.Count -eq 0) { return $null }
    return $parsedEnvironment
}

function Get-AmbientEnvironmentHashtable {
    $ambientEnvironment = @{}
    foreach ($entry in @(Get-ChildItem env:)) {
        $ambientEnvironment[$entry.Name] = "$($entry.Value)"
    }
    return $ambientEnvironment
}

function Set-SessionEnvironmentFromHashtable {
    param([Parameter(Mandatory = $true)][hashtable]$Environment)
    foreach ($key in @($Environment.Keys)) {
        Set-Item -LiteralPath "env:$key" -Value "$($Environment[$key])"
    }
}

function Invoke-RequiredVcvarsCapture {
    param([Parameter(Mandatory = $true)][hashtable]$InitialEnvironment)

    $captureOverride = Get-SoakTestDependencyOverride -Name "captureVcvarsEnvironment"
    if ($null -ne $captureOverride) { return & $captureOverride $InitialEnvironment }

    $commandLine = '/d /s /c ""{0}" {1} && set"' -f $script:RequiredVcvarsBatch, $script:RequiredVcvarsArguments
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = if ($env:ComSpec) { $env:ComSpec } else { "cmd.exe" }
    $startInfo.Arguments = $commandLine
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.CreateNoWindow = $true
    foreach ($key in @($InitialEnvironment.Keys)) {
        $startInfo.EnvironmentVariables[[string]$key] = "$($InitialEnvironment[$key])"
    }
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    if (-not $process.Start()) { return $null }
    $standardOutputTask = $process.StandardOutput.ReadToEndAsync()
    $standardErrorTask = $process.StandardError.ReadToEndAsync()
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) { return $null }
    return ConvertFrom-CommandLineSetOutput -Output $standardOutputTask.Result
}

function Get-LinkersWithWhere {
    param([hashtable]$Environment = (@{}))

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = "where.exe"
    $startInfo.Arguments = "link.exe"
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.CreateNoWindow = $true
    foreach ($key in @($Environment.Keys)) {
        $startInfo.EnvironmentVariables[[string]$key] = "$($Environment[$key])"
    }
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    if (-not $process.Start()) {
        throw "Refusing the Windows native soak build because where.exe link.exe failed to start."
    }
    $standardOutputTask = $process.StandardOutput.ReadToEndAsync()
    $standardErrorTask = $process.StandardError.ReadToEndAsync()
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) {
        throw "Refusing the Windows native soak build because where.exe link.exe failed (exit $($process.ExitCode))."
    }
    $locatedLinkers = @(
        @($standardOutputTask.Result -split "\r?\n") |
            ForEach-Object { $_.Trim() } |
            Where-Object { $_ }
    )
    if ($locatedLinkers.Count -eq 0) {
        throw "Refusing the Windows native soak build because where.exe link.exe returned no linker."
    }
    return $locatedLinkers
}

function Assert-ExactMsvcToolset {
    param(
        [Parameter(Mandatory = $true)][hashtable]$Environment,
        [Parameter(Mandatory = $true)][scriptblock]$FileIsRegular
    )
    $vcToolsInstallDir = "$($Environment["VCToolsInstallDir"])".Trim()
    if (-not $vcToolsInstallDir) {
        throw "Refusing the Windows native soak build without VCToolsInstallDir. Initialize the exact x64 Visual Studio 2022 Community environment for MSVC $($script:RequiredVcToolsVersion)."
    }
    $resolvedLinker = Get-MsvcHostLinkerPath -VcToolsInstallDir $vcToolsInstallDir
    if ((ConvertTo-PathComparisonKey $resolvedLinker) -ne (ConvertTo-PathComparisonKey $script:RequiredMsvcLinker)) {
        throw "Refusing the Windows native soak build because the resolved MSVC toolset does not match the required Visual Studio 2022 Community $($script:RequiredVcToolsVersion) toolset. Resolved linker: '$resolvedLinker'. Required linker: '$($script:RequiredMsvcLinker)'."
    }
    if (-not (& $FileIsRegular $resolvedLinker)) {
        throw "Refusing the Windows native soak build because the asserted MSVC linker is missing: $resolvedLinker"
    }
    return $resolvedLinker
}

function Assert-ExactMsvcLinkerFirst {
    param(
        [Parameter(Mandatory = $true)][hashtable]$Environment,
        [Parameter(Mandatory = $true)][scriptblock]$LocateLinkers
    )
    $locatedLinkers = @(& $LocateLinkers $Environment)
    if ($locatedLinkers.Count -eq 0) {
        throw "Refusing the Windows native soak build because where.exe link.exe returned no linker."
    }
    $firstLinker = "$($locatedLinkers[0])"
    if ((ConvertTo-PathComparisonKey $firstLinker) -ne (ConvertTo-PathComparisonKey $script:RequiredMsvcLinker)) {
        throw "Refusing the Windows native soak build because where.exe link.exe resolves '$firstLinker' first; required first linker: '$($script:RequiredMsvcLinker)'."
    }
    return $locatedLinkers
}

function Get-VerifiedNativeBuildEnvironment {
    param(
        [Parameter(Mandatory = $true)][hashtable]$Environment,
        [Parameter(Mandatory = $true)][bool]$IsWindowsHost,
        [Parameter(Mandatory = $true)][scriptblock]$FileIsRegular,
        [Parameter(Mandatory = $true)][scriptblock]$LocateLinkers
    )
    if (-not $IsWindowsHost) { return $Environment }

    $linker = Assert-ExactMsvcToolset -Environment $Environment -FileIsRegular $FileIsRegular

    # Pin the target linker so Cargo can never fall through to Git usr/bin/link.exe.
    $verifiedEnvironment = @{}
    foreach ($key in @($Environment.Keys)) {
        $verifiedEnvironment[$key] = $Environment[$key]
    }
    $verifiedEnvironment["CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER"] = $linker

    $locatedLinkers = @(Assert-ExactMsvcLinkerFirst -Environment $verifiedEnvironment -LocateLinkers $LocateLinkers)
    Write-Host "[run-soak] pinned CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=$linker"
    Write-Host "[run-soak] where.exe link.exe:"
    foreach ($locatedLinker in $locatedLinkers) {
        Write-Host "[run-soak]   $locatedLinker"
    }
    return $verifiedEnvironment
}

function Invoke-SoakMsvcPreflight {
    param([Parameter(Mandatory = $true)][hashtable]$Environment)

    $fileIsRegular = Resolve-SoakDependency -Name "fileIsRegular" -Default ${function:Test-RegularFilePath}
    $locateLinkers = Resolve-SoakDependency -Name "locateLinkers" -Default ${function:Get-LinkersWithWhere}
    try {
        return Get-VerifiedNativeBuildEnvironment -Environment $Environment -IsWindowsHost $true `
            -FileIsRegular $fileIsRegular -LocateLinkers $locateLinkers
    } catch {
        $ambientReason = $_.Exception.Message
        $capturedEnvironment = Invoke-RequiredVcvarsCapture -InitialEnvironment $Environment
        if ($null -eq $capturedEnvironment) {
            throw "Refusing the Windows native soak build because automatic vcvars64.bat $($script:RequiredVcvarsArguments) initialization failed; run from an x64 Visual Studio 2022 Community Developer Command Prompt. Ambient reason: $ambientReason"
        }
        return Get-VerifiedNativeBuildEnvironment -Environment $capturedEnvironment -IsWindowsHost $true `
            -FileIsRegular $fileIsRegular -LocateLinkers $locateLinkers
    }
}

if ($null -ne $global:SYNDOCAL_RUN_SOAK_TEST_DEPENDENCIES -and -not $PreflightOnly) {
    throw "SYNDOCAL_RUN_SOAK_TEST_DEPENDENCIES is a hermetic -PreflightOnly self-test seam and must never be present on a build or soak-harness run."
}

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$isWindowsHost = $IsWindows -or $env:OS -eq "Windows_NT"

if (((-not $SkipBuild) -or $PreflightOnly) -and $isWindowsHost) {
    $ambientEnvironment = Get-AmbientEnvironmentHashtable
    $verifiedEnvironment = Invoke-SoakMsvcPreflight -Environment $ambientEnvironment
    Set-SessionEnvironmentFromHashtable -Environment $verifiedEnvironment
}

if ($PreflightOnly) {
    if (-not $isWindowsHost) {
        Write-Host "[run-soak] preflight-only: non-Windows host; the exact MSVC linker gate does not apply."
    }
    Write-Host "[run-soak] preflight-only completed before any Cargo or soak-harness launch."
    return
}

if (-not $SkipBuild) {
    $cargoArguments = @("build", "-p", "engine", "--example", "syndocal_soak", "--locked")
    if ($Configuration -eq "Release") { $cargoArguments += "--release" }
    $cargoOverride = Get-SoakTestDependencyOverride -Name "invokeCargo"
    if ($null -ne $cargoOverride) {
        & $cargoOverride $cargoArguments
    } else {
        & cargo @cargoArguments
    }
    if ($LASTEXITCODE -ne 0) { throw "Failed to build the soak harness." }
}

$suffix = if ($IsWindows -or $env:OS -eq "Windows_NT") { ".exe" } else { "" }
$profileDirectory = $Configuration.ToLowerInvariant()
$executable = Join-Path $root "target/$profileDirectory/examples/syndocal_soak$suffix"
if (-not (Test-Path -LiteralPath $executable)) {
    throw "Soak harness not found: $executable"
}

$resolvedReport = [System.IO.Path]::GetFullPath((Join-Path $root $ReportPath))
$reportDirectory = Split-Path -Parent $resolvedReport
New-Item -ItemType Directory -Force -Path $reportDirectory | Out-Null

$harnessArguments = @(
    "--duration-seconds", $DurationSeconds,
    "--report", $resolvedReport
)
if ($MixedLighting) {
    $harnessArguments += "--mixed-lighting"
}

$process = Start-Process -FilePath $executable -ArgumentList $harnessArguments -PassThru -NoNewWindow

$peakWorkingSet = 0L
$peakCpuSeconds = 0.0
try {
    while (-not $process.HasExited) {
        $process.Refresh()
        $peakWorkingSet = [Math]::Max($peakWorkingSet, $process.WorkingSet64)
        $peakCpuSeconds = [Math]::Max($peakCpuSeconds, $process.TotalProcessorTime.TotalSeconds)
        Start-Sleep -Seconds $SampleIntervalSeconds
    }
    $process.WaitForExit()
} finally {
    if (-not $process.HasExited) {
        Stop-Process -Id $process.Id -Force
    }
}

if ($process.ExitCode -ne 0) {
    throw "Soak harness exited with code $($process.ExitCode)."
}
if (-not (Test-Path -LiteralPath $resolvedReport)) {
    throw "Soak harness did not produce a report."
}

$report = Get-Content -LiteralPath $resolvedReport -Raw | ConvertFrom-Json
$report | Add-Member -NotePropertyName process_peak_working_set_bytes -NotePropertyValue $peakWorkingSet -Force
$report | Add-Member -NotePropertyName process_cpu_seconds -NotePropertyValue $peakCpuSeconds -Force
$report | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $resolvedReport -Encoding utf8

Write-Host "Syndocal soak passed: $resolvedReport"
Write-Host "Peak working set: $([Math]::Round($peakWorkingSet / 1MB, 1)) MB"
