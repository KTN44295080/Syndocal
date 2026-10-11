param([switch]$ValidateOnly)
$ErrorActionPreference = 'Stop'
if (Get-Process -Name syndocal -ErrorAction SilentlyContinue) { throw 'Close Syndocal before uninstalling.' }
. (Join-Path $PSScriptRoot 'SyndocalInstallations.ps1')
$taskActions = @(Get-SyndocalInstallations)
if ($ValidateOnly) { Write-Output ('PASS: uninstall registrations verified; count=' + $taskActions.Count); exit 0 }
Write-Output 'This removes Syndocal and ALL its settings/recovery data. External projects/media are kept.'
if ((Read-Host 'Type REMOVE to continue') -cne 'REMOVE') { Write-Output 'Cancelled'; exit 0 }
foreach ($taskAction in $taskActions) {
    $taskParameters = @{FilePath=$taskAction.File;WindowStyle='Normal';Wait=$true;PassThru=$true}
    if ($taskAction.Args.Count) { $taskParameters.ArgumentList = $taskAction.Args }
    $taskProcess = Start-Process @taskParameters
    if ($taskProcess.ExitCode -ne 0) { throw ('Uninstall incomplete (cancel/restart/failure): ' + $taskProcess.ExitCode) }
}
& (Join-Path $PSScriptRoot 'Remove-SyndocalData.ps1')
exit $LASTEXITCODE
