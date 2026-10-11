param([switch]$ValidateOnly)
$ErrorActionPreference = 'Stop'
$taskApp = Join-Path $PSScriptRoot 'Syndocal_1.2.0-alpha.73_x64-setup.exe'
$taskMsi = Join-Path $PSScriptRoot 'Syndocal_1.2.0-alpha.73_x64_ja-JP.msi'
$taskRuntime = Join-Path $PSScriptRoot 'VC_redist.x64.exe'
$taskExpected = @{
 'Syndocal_1.2.0-alpha.73_x64-setup.exe' = 'e51d0461432a0135a6776e6ce9d4c5b721a9c3dea1a3d049c1a96ebeb9a1f786'
 'Syndocal_1.2.0-alpha.73_x64_ja-JP.msi' = '497edacd5057aa1279e5a6217772aee363980aad535fa400f517f4fcfdb10f35'
 'VC_redist.x64.exe' = '843068991daaa1f73ad9f6239bce4d0f6a07a51f18c37ea2a867e9beca71295c'
}
foreach ($taskName in $taskExpected.Keys) {
 $taskPath = Join-Path $PSScriptRoot $taskName
 if ((Get-FileHash -LiteralPath $taskPath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $taskExpected[$taskName]) {
  throw 'Setup files could not be verified. Obtain the original package again.'
 }
}
$taskSignature = Get-AuthenticodeSignature -FilePath $taskRuntime
if ($taskSignature.Status -ne 'Valid' -or $taskSignature.SignerCertificate.Subject -notmatch 'Microsoft Corporation') {
 throw 'The Microsoft runtime signature could not be verified.'
}
if ($ValidateOnly) { Write-Output 'PASS: exact Syndocal installer hash and signed Microsoft prerequisite; no installation performed.'; exit 0 }
if (-not [Environment]::Is64BitOperatingSystem) { throw 'This package requires 64-bit Windows 10 or later.' }
if (Get-Process -Name syndocal -ErrorAction SilentlyContinue) { throw 'Close Syndocal before updating.' }
. (Join-Path $PSScriptRoot 'SyndocalInstallations.ps1')
$taskPrevious = @(Get-SyndocalInstallations)
if ($taskPrevious.Count -gt 1) { throw 'Multiple Syndocal installations found. Remove the duplicate before updating.' }
$taskRuntimeProcess = Start-Process -FilePath $taskRuntime -ArgumentList '/install','/passive','/norestart' -WindowStyle Normal -Wait -PassThru
if ($taskRuntimeProcess.ExitCode -eq 3010) {
 Write-Output 'Restart Windows, then run Install-Windows.cmd again.'; exit 3010
}
if ($taskRuntimeProcess.ExitCode -eq 1638) {
 $taskInstalledRuntime = Get-ItemProperty -LiteralPath 'HKLM:\SOFTWARE\Microsoft\VisualStudio\14.0\VC\Runtimes\x64'
 if ($taskInstalledRuntime.Installed -ne 1 -or [version]$taskInstalledRuntime.Version.TrimStart('v') -lt [version]'14.44.35207.0') {
  throw 'A compatible Microsoft C++ x64 runtime could not be confirmed.'
 }
} elseif ($taskRuntimeProcess.ExitCode -ne 0) {
 throw ('Microsoft runtime installation failed: ' + $taskRuntimeProcess.ExitCode)
}
$taskSetupArguments = @()
if ($taskPrevious.Count -eq 1 -and $taskPrevious[0].Kind -eq 'nsis') {
 $taskSetupArguments = @('/UPDATE')
}
$taskParameters = @{FilePath=$taskApp;WindowStyle='Normal';Wait=$true;PassThru=$true}
if ($taskPrevious.Count -eq 1 -and $taskPrevious[0].Kind -eq 'msi') {
 $taskParameters.FilePath = Join-Path $env:SystemRoot 'System32\msiexec.exe'
 $taskSetupArguments = @('/i', ('"' + $taskMsi + '"'), 'REBOOT=ReallySuppress')
}
if ($taskSetupArguments.Count) { $taskParameters.ArgumentList = $taskSetupArguments }
$taskSetupProcess = Start-Process @taskParameters
exit $taskSetupProcess.ExitCode
