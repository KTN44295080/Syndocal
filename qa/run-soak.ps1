param(
    [ValidateRange(1, 86400)]
    [int]$DurationSeconds = 3600,
    [ValidateRange(1, 60)]
    [int]$SampleIntervalSeconds = 1,
    [string]$ReportPath = "target/qa/m5-soak.json",
    [ValidateSet("Debug", "Release")]
    [string]$Configuration = "Release",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if (-not $SkipBuild) {
    $cargoArguments = @("build", "-p", "engine", "--example", "syndocal_soak", "--locked")
    if ($Configuration -eq "Release") { $cargoArguments += "--release" }
    & cargo @cargoArguments
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

$process = Start-Process -FilePath $executable -ArgumentList @(
    "--duration-seconds", $DurationSeconds,
    "--report", $resolvedReport
) -PassThru -NoNewWindow

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
