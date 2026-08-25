# Focused clean-break test for the retired single-window 4K runner.
# It does not enumerate a real window or process and performs no mutation.

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$runnerPath = Join-Path $PSScriptRoot "run-syndocal-native-4k-acceptance.ps1"
. $runnerPath

$checks = [System.Collections.Generic.List[object]]::new()
$message = Get-RetiredNative4kAcceptanceMessage
$checks.Add([pscustomobject]@{
  Name = "retired entrypoint names the three-display replacement"
  Passed = ($message -match "three-display-show-acceptance" -and $message -match "DISPLAY2" -and $message -match "DISPLAY5" -and $message -match "DISPLAY3")
  Detail = $message
})

try {
  Invoke-RetiredNative4kAcceptance
  $checks.Add([pscustomobject]@{ Name = "retired entrypoint fails closed"; Passed = $false; Detail = "did not throw" })
} catch {
  $checks.Add([pscustomobject]@{
    Name = "retired entrypoint fails closed"
    Passed = $_.Exception.Message -match "Fail closed" -and $_.Exception.Message -match "retired"
    Detail = $_.Exception.Message
  })
}

try {
  Invoke-RetiredNative4kAcceptance -Arguments @("-Apply", "-ExpectedMonitorIdentifier", "MSI3DD2")
  $checks.Add([pscustomobject]@{ Name = "copied legacy flags still receive replacement"; Passed = $false; Detail = "did not throw" })
} catch {
  $checks.Add([pscustomobject]@{
    Name = "copied legacy flags still receive replacement"
    Passed = $_.Exception.Message -match "three-display-show-acceptance" -and $_.Exception.Message -match "Retired arguments are not accepted"
    Detail = $_.Exception.Message
  })
}

$source = [IO.File]::ReadAllText($runnerPath)
$forbidden = @("ShowWindow", "SetWindowPos", "SetThreadDpiAwarenessContext", "Get-Process", "Start-Process", "Stop-Process", "SendInput")
$found = @($forbidden | Where-Object { $source.Contains($_) })
$checks.Add([pscustomobject]@{
  Name = "retired entrypoint contains no native operation path"
  Passed = $found.Count -eq 0
  Detail = if ($found.Count -eq 0) { "no legacy native-operation tokens" } else { "forbidden token(s): $($found -join ', ')" }
})

foreach ($check in $checks) {
  $status = if ($check.Passed) { "PASS" } else { "FAIL" }
  "$status $($check.Name) -- $($check.Detail)"
}
$failed = @($checks | Where-Object { -not $_.Passed }).Count
"SUMMARY: $($checks.Count) checks, $failed failed"
if ($failed -gt 0) { exit 1 }
