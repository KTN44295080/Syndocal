param([Parameter(Mandatory)][string]$BaselineRoot,[Parameter(Mandatory)][string]$CurrentRoot,[Parameter(Mandatory)][string]$EvidencePath)
$ErrorActionPreference='Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw 'This installs/removes products and is restricted to a disposable GitHub-hosted Windows runner.' }
. (Join-Path $PSScriptRoot 'SyndocalInstallations.ps1')
if (@(Get-SyndocalInstallations).Count -ne 0 -or (Get-Process -Name syndocal -ErrorAction SilentlyContinue)) { throw 'Expected a clean runner' }
$taskRoots=@((Join-Path $env:APPDATA 'jp.seraf.ktn.syndocal'),(Join-Path $env:LOCALAPPDATA 'jp.seraf.ktn.syndocal'))
foreach ($taskRoot in $taskRoots) { if (Test-Path -LiteralPath $taskRoot) { throw 'Unexpected preexisting product data' } }
$taskChecks=New-Object 'System.Collections.Generic.List[string]'
$taskReport=[ordered]@{passed=$false;sourceCommit=$env:GITHUB_SHA;currentRunId=$env:CURRENT_RUN_ID;baselineRunId='38087767111';checks=$taskChecks;physicalOutput=$false}
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class LifecycleVault {
 [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Unicode)] public struct Credential {
  public uint Flags, Type; public string TargetName, Comment; public System.Runtime.InteropServices.ComTypes.FILETIME LastWritten;
  public uint CredentialBlobSize; public IntPtr CredentialBlob; public uint Persist, AttributeCount; public IntPtr Attributes; public string TargetAlias, UserName;
 }
 [DllImport("advapi32.dll",EntryPoint="CredWriteW",CharSet=CharSet.Unicode,SetLastError=true)] public static extern bool Write(ref Credential credential,uint flags);
 [DllImport("advapi32.dll",EntryPoint="CredReadW",CharSet=CharSet.Unicode,SetLastError=true)] public static extern bool Read(string target,uint type,uint flags,out IntPtr credential);
 [DllImport("advapi32.dll",EntryPoint="CredFree")] public static extern void Free(IntPtr credential);
}
'@
$taskCredentialNames=@('jp.seraf.ktn.syndocal/dj-link/v1','jp.seraf.ktn.syndocal/dj-link/v1/rollback',('Syndocal/AgentAuthority/v1/'+('c'*64)))
function Assert-NoCredentials {
 foreach ($taskName in $taskCredentialNames) {
  $taskPointer=[IntPtr]::Zero
  if ([LifecycleVault]::Read($taskName,1,0,[ref]$taskPointer)) { [LifecycleVault]::Free($taskPointer); throw 'Product credential unexpectedly retained' }
  Assert-Condition ([Runtime.InteropServices.Marshal]::GetLastWin32Error() -eq 1168) ('Credential absent '+$taskName)
 }
}
function Assert-Condition($Condition,[string]$Name) { if (-not $Condition) { throw $Name }; $taskChecks.Add($Name) }
function Find-Installer([string]$Root,[string]$Name) {
 $taskFiles=@(Get-ChildItem -LiteralPath $Root -Recurse -File | Where-Object Name -eq $Name)
 if ($taskFiles.Count -ne 1) { throw "Expected one $Name" }; return $taskFiles[0].FullName
}
function Run-Installer([string]$File,[string[]]$Arguments,[int]$Expected=0) {
 $taskProcess=Start-Process -FilePath $File -ArgumentList $Arguments -Wait -PassThru -WindowStyle Hidden
 Assert-Condition ($taskProcess.ExitCode -eq $Expected) ("Installer exit $Expected for " + [IO.Path]::GetFileName($File))
}
function Set-Sentinels {
 Assert-NoCredentials
 foreach ($taskName in $taskCredentialNames) {
  $taskCredential=New-Object LifecycleVault+Credential; $taskCredential.Type=1; $taskCredential.TargetName=$taskName; $taskCredential.Persist=1; $taskCredential.UserName='lifecycle-fixture'
  Assert-Condition ([LifecycleVault]::Write([ref]$taskCredential,0)) ('Created owned test credential '+$taskName)
 }
 foreach ($taskRoot in $taskRoots) { New-Item -ItemType Directory -Path $taskRoot -Force | Out-Null; Set-Content -LiteralPath (Join-Path $taskRoot 'lifecycle-sentinel') -Value 'preserve-this-user-setting' }
}
function Assert-Sentinels {
 foreach ($taskRoot in $taskRoots) { Assert-Condition ((Get-Content -LiteralPath (Join-Path $taskRoot 'lifecycle-sentinel') -Raw).Trim() -eq 'preserve-this-user-setting') ('Update preserved '+$taskRoot) }
}
function Assert-Removed([string]$Executable) {
 $taskLimit=(Get-Date).AddSeconds(30)
 while ((Test-Path -LiteralPath $Executable) -and (Get-Date) -lt $taskLimit) { Start-Sleep -Milliseconds 250 }
 Assert-Condition (-not (Test-Path -LiteralPath $Executable)) 'Product executable removed'
 Assert-Condition (@(Get-SyndocalInstallations).Count -eq 0) 'Product uninstall registration removed'
 & (Join-Path $PSScriptRoot 'Remove-SyndocalData.ps1')
 Assert-NoCredentials
 foreach ($taskRoot in $taskRoots) { Assert-Condition (-not (Test-Path -LiteralPath $taskRoot)) ('Complete removal purged '+$taskRoot) }
 Assert-Condition (-not (Test-Path -LiteralPath 'HKCU:\Software\Seraf() / KTN\Syndocal')) 'Installer preference registry removed'
 Assert-Condition ((Get-Content -LiteralPath $taskForeign -Raw).Trim() -eq 'external-project') 'External project retained'
}
$taskOldNsis=Find-Installer $BaselineRoot 'Syndocal_1.2.0-alpha.72_x64-setup.exe'
$taskNewNsis=Find-Installer $CurrentRoot 'Syndocal_1.2.0-alpha.73_x64-setup.exe'
$taskOldMsi=Find-Installer $BaselineRoot 'Syndocal_1.2.0-alpha.72_x64_ja-JP.msi'
$taskNewMsi=Find-Installer $CurrentRoot 'Syndocal_1.2.0-alpha.73_x64_ja-JP.msi'
$taskReport.installerHashes=@{}; foreach ($taskFile in @($taskOldNsis,$taskNewNsis,$taskOldMsi,$taskNewMsi)) { $taskReport.installerHashes[[IO.Path]::GetFileName($taskFile)]=(Get-FileHash -LiteralPath $taskFile -Algorithm SHA256).Hash.ToLowerInvariant() }
$taskInstall=Join-Path $env:RUNNER_TEMP 'SyndocalLifecycle'
$taskForeign=Join-Path $env:RUNNER_TEMP 'syndocal-lifecycle-external.sdc'; Set-Content -LiteralPath $taskForeign -Value 'external-project'
$taskMsiExec=Join-Path $env:SystemRoot 'System32\msiexec.exe'
try {
 Run-Installer $taskOldNsis @('/S',('/D='+$taskInstall))
 Set-Sentinels
 Run-Installer $taskNewNsis @('/S','/UPDATE',('/D='+$taskInstall))
 Assert-Condition (Test-Path -LiteralPath (Join-Path $taskInstall 'syndocal.exe')) 'Updated NSIS application exists'
 Assert-Sentinels
 $taskInstalled=@(Get-SyndocalInstallations)
 Assert-Condition ($taskInstalled.Count -eq 1 -and $taskInstalled[0].Kind -eq 'nsis') 'NSIS update kept one installation'
 Run-Installer $taskInstalled[0].File @('/S')
 Assert-Removed (Join-Path $taskInstall 'syndocal.exe')
 Run-Installer $taskMsiExec @('/i',('"'+$taskOldMsi+'"'),'/qn','REBOOT=ReallySuppress')
 Set-Sentinels
 Run-Installer $taskMsiExec @('/i',('"'+$taskNewMsi+'"'),'/qn','REBOOT=ReallySuppress')
 Assert-Condition (Test-Path -LiteralPath (Join-Path $env:ProgramFiles 'Syndocal\syndocal.exe')) 'Updated MSI application exists'
 Assert-Sentinels
 $taskInstalled=@(Get-SyndocalInstallations)
 Assert-Condition ($taskInstalled.Count -eq 1 -and $taskInstalled[0].Kind -eq 'msi') 'MSI upgrade replaced old product without duplicate'
 Run-Installer $taskMsiExec @('/x',$taskInstalled[0].Args[1],'/qn','REBOOT=ReallySuppress')
 Assert-Removed (Join-Path $env:ProgramFiles 'Syndocal\syndocal.exe')
 $taskReport.passed=$true
} catch { $taskReport.error=$_.Exception.Message; throw }
finally { $taskReport | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $EvidencePath -Encoding utf8 }
