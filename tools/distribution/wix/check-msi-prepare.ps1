param([Parameter(Mandatory)][string]$InstallerPath,[Parameter(Mandatory)][string]$EvidencePath)
$ErrorActionPreference='Stop'
Add-Type -TypeDefinition @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class SyndocalMsiPrepare {
 [DllImport("msi.dll",CharSet=CharSet.Unicode)] public static extern uint MsiOpenPackageExW(string package,uint options,out uint handle);
 [DllImport("msi.dll",CharSet=CharSet.Unicode)] public static extern uint MsiDoActionW(uint handle,string action);
 [DllImport("msi.dll",CharSet=CharSet.Unicode)] public static extern uint MsiSetPropertyW(uint handle,string name,string value);
 [DllImport("msi.dll",CharSet=CharSet.Unicode)] static extern uint MsiGetPropertyW(uint handle,string name,StringBuilder value,ref uint length);
 [DllImport("msi.dll")] public static extern uint MsiSetInternalUI(uint level,IntPtr window);
 [DllImport("msi.dll")] public static extern uint MsiCloseHandle(uint handle);
 [DllImport("msi.dll",CharSet=CharSet.Unicode)] public static extern uint MsiEnableLogW(uint mode,string path,uint attributes);
 public static string Read(uint handle,string name){uint size=32768;var value=new StringBuilder((int)size);uint result=MsiGetPropertyW(handle,name,value,ref size);if(result!=0)throw new Exception("Property read failed: "+result);return value.ToString();}
}
'@
$taskSettings=Join-Path $env:LOCALAPPDATA 'jp.seraf.ktn.syndocal/extensions.ini'
$taskBefore=if(Test-Path -LiteralPath $taskSettings){(Get-FileHash -LiteralPath $taskSettings -Algorithm SHA256).Hash}else{'absent'}
$taskChecks=New-Object 'System.Collections.Generic.List[string]'
$taskPreviousUi=[SyndocalMsiPrepare]::MsiSetInternalUI(2,[IntPtr]::Zero)
function Assert-Check($Condition,[string]$Name){if(-not $Condition){throw $Name};$taskChecks.Add($Name)}
[void][SyndocalMsiPrepare]::MsiEnableLogW(8191,(Join-Path $env:TEMP "syndocal-alpha80-msi-prepare.log"),2)
try {
 foreach($taskChoice in @('0','1')) {
  $taskHandle=[uint32]0
  try {
   Assert-Check ([SyndocalMsiPrepare]::MsiOpenPackageExW([IO.Path]::GetFullPath($InstallerPath),0,[ref]$taskHandle) -eq 0) ('Open read-only MSI session '+$taskChoice)
   Assert-Check ([SyndocalMsiPrepare]::MsiSetPropertyW($taskHandle,'DJLINK',$taskChoice) -eq 0) ('Set native selection '+$taskChoice)
   Assert-Check ([SyndocalMsiPrepare]::MsiDoActionW($taskHandle,'SyndocalPrepareExtensions') -eq 0) ('Embedded prepare action succeeds '+$taskChoice)
   $taskExpected=if($taskChoice -eq '1'){'1'}else{''}
   Assert-Check ([SyndocalMsiPrepare]::Read($taskHandle,'SYNDOCAL_DJLINK_SELECTED') -ceq $taskExpected) ('Native checkbox initialized '+$taskChoice)
   # The prepare action must preserve a later checkbox edit, including unchecking.
   $taskEdit=if($taskChoice -eq '1'){''}else{'1'}
   Assert-Check ([SyndocalMsiPrepare]::MsiSetPropertyW($taskHandle,'SYNDOCAL_DJLINK_SELECTED',$taskEdit) -eq 0) ('Edit native checkbox '+$taskChoice)
   Assert-Check ([SyndocalMsiPrepare]::MsiDoActionW($taskHandle,'SyndocalPrepareExtensions') -eq 0 -and [SyndocalMsiPrepare]::Read($taskHandle,'SYNDOCAL_DJLINK_SELECTED') -ceq $taskEdit) ('Prepare preserves checkbox edits '+$taskChoice)
   Assert-Check ([SyndocalMsiPrepare]::MsiDoActionW($taskHandle,'SyndocalPlanExtensions') -eq 0) ('Embedded transaction planning succeeds '+$taskChoice)
   $taskPlan=[SyndocalMsiPrepare]::Read($taskHandle,'SyndocalWriteExtensions')
   Assert-Check ($taskPlan.StartsWith($(if($taskEdit -eq '1'){"1`n"}else{"0`n"}))) ('Plan uses the edited checkbox '+$taskChoice)
  } finally {if($taskHandle -ne 0){[void][SyndocalMsiPrepare]::MsiCloseHandle($taskHandle)}}
 }
 $taskAfter=if(Test-Path -LiteralPath $taskSettings){(Get-FileHash -LiteralPath $taskSettings -Algorithm SHA256).Hash}else{'absent'}
 Assert-Check ($taskBefore -ceq $taskAfter) 'Prepare and plan never mutate user settings'
} finally {[void][SyndocalMsiPrepare]::MsiEnableLogW(0,$null,0);[void][SyndocalMsiPrepare]::MsiSetInternalUI($taskPreviousUi,[IntPtr]::Zero)}
[pscustomobject]@{passed=$true;installer=[IO.Path]::GetFullPath($InstallerPath);sha256=(Get-FileHash -LiteralPath $InstallerPath -Algorithm SHA256).Hash.ToLowerInvariant();checks=$taskChecks;noProductInstallation=$true} | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $EvidencePath -Encoding utf8
Write-Output ('PASS embedded native MSI prepare/plan: '+$taskChecks.Count+' checks; no installation or settings write')
