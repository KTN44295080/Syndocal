param([Parameter(Mandatory)][string]$InstallerPath,[Parameter(Mandatory)][string]$EvidencePath)
$ErrorActionPreference='Stop'
$taskInstaller=New-Object -ComObject WindowsInstaller.Installer
$taskDb=$taskInstaller.GetType().InvokeMember('OpenDatabase','InvokeMethod',$null,$taskInstaller,@([IO.Path]::GetFullPath($InstallerPath),0))
function Read-Rows([string]$Query,[int]$Columns) {
 $taskView=$taskDb.GetType().InvokeMember('OpenView','InvokeMethod',$null,$taskDb,@($Query))
 [void]$taskView.GetType().InvokeMember('Execute','InvokeMethod',$null,$taskView,$null)
 $taskRows=@()
 while($taskRow=$taskView.GetType().InvokeMember('Fetch','InvokeMethod',$null,$taskView,$null)) {
  $taskCells=@();for($taskColumn=1;$taskColumn -le $Columns;$taskColumn++){$taskCells+= $taskRow.GetType().InvokeMember('StringData','GetProperty',$null,$taskRow,@($taskColumn))}
  $taskRows+=,@($taskCells)
 }
 [void]$taskView.GetType().InvokeMember('Close','InvokeMethod',$null,$taskView,$null)
 return ,$taskRows
}
$taskChecks=New-Object 'System.Collections.Generic.List[string]'
function Assert-Check($Condition,[string]$Name) {if(-not $Condition){throw $Name};$taskChecks.Add($Name)}
$taskProperties=@{};foreach($taskRow in (Read-Rows 'SELECT Property, Value FROM Property' 2)){$taskProperties[$taskRow[0]]=$taskRow[1]}
Assert-Check ($taskProperties.ProductVersion -eq '1.2.80' -and $taskProperties.ProductName -eq 'Syndocal') 'MSI product alpha.80'
Assert-Check (-not $taskProperties.ContainsKey('ARPNOREPAIR')) 'Native maintenance Repair remains visible and enabled'
$taskControl=Read-Rows 'SELECT Dialog_, Control, Type, Property, Text FROM Control' 5
Assert-Check (@($taskControl | Where-Object {$_[0] -eq 'SyndocalExtensionsDlg' -and $_[1] -eq 'DJLink' -and $_[2] -eq 'CheckBox' -and $_[3] -eq 'SYNDOCAL_DJLINK_SELECTED' -and $_[4] -eq 'DJ Link'}).Count -eq 1) 'Native MSI DJ Link checkbox'
$taskEvents=Read-Rows 'SELECT Dialog_, Control_, Event, Argument, Condition, Ordering FROM ControlEvent' 6
foreach($taskTransition in @(@('WelcomeDlg','Next','SyndocalExtensionsDlg'),@('InstallDirDlg','Back','SyndocalExtensionsDlg'),@('MaintenanceTypeDlg','RepairButton','SyndocalExtensionsDlg'),@('SyndocalExtensionsDlg','Next','InstallDirDlg'),@('SyndocalExtensionsDlg','Next','VerifyReadyDlg'))) {
 Assert-Check (@($taskEvents|Where-Object {$_[0] -eq $taskTransition[0] -and $_[1] -eq $taskTransition[1] -and $_[2] -eq 'NewDialog' -and $_[3] -eq $taskTransition[2]}).Count -eq 1) ('Native MSI navigation '+($taskTransition -join '/'))
}
$taskActions=Read-Rows 'SELECT Action, Type, Source, Target FROM CustomAction' 4
foreach($taskAction in @(@('SyndocalPrepareExtensions',1,'PrepareExtensions'),@('SyndocalPlanExtensions',1,'PlanExtensions'),@('SyndocalRollbackExtensions',1281,'RollbackExtensions'),@('SyndocalWriteExtensions',1025,'WriteExtensions'),@('SyndocalCommitExtensions',1537,'CommitExtensions'))) {
 Assert-Check (@($taskActions|Where-Object {$_[0] -eq $taskAction[0] -and [int]$_[1] -eq $taskAction[1] -and $_[2] -eq 'SyndocalExtensionsCA' -and $_[3] -eq $taskAction[2]}).Count -eq 1) ('Embedded MSI action '+$taskAction[0])
}
$taskSequence=@{};foreach($taskRow in (Read-Rows 'SELECT Action, Condition, Sequence FROM InstallExecuteSequence' 3)){$taskSequence[$taskRow[0]]=$taskRow}
Assert-Check ([int]$taskSequence.SyndocalPrepareExtensions[2] -lt [int]$taskSequence.CostInitialize[2]) 'Validate existing settings before costing'
Assert-Check ([int]$taskSequence.SyndocalPlanExtensions[2] -lt [int]$taskSequence.InstallInitialize[2]) 'Plan before installation mutation'
Assert-Check ([int]$taskSequence.SyndocalRollbackExtensions[2] -lt [int]$taskSequence.SyndocalWriteExtensions[2] -and [int]$taskSequence.SyndocalWriteExtensions[2] -lt [int]$taskSequence.SyndocalCommitExtensions[2] -and [int]$taskSequence.SyndocalCommitExtensions[2] -lt [int]$taskSequence.InstallFinalize[2]) 'Rollback/write/commit ordering'
foreach($taskName in @('SyndocalPrepareExtensions','SyndocalPlanExtensions','SyndocalRollbackExtensions','SyndocalWriteExtensions','SyndocalCommitExtensions')){Assert-Check ($taskSequence[$taskName][1] -eq 'NOT REMOVE') ('No extension write on uninstall '+$taskName)}
$taskSecure=$taskProperties.SecureCustomProperties.Split(';')
foreach($taskName in @('DJLINK','SYNDOCAL_DJLINK_SELECTED','SYNDOCAL_CHOICES_READY')){Assert-Check ($taskSecure -contains $taskName) ('Secure property '+$taskName)}
[pscustomobject]@{passed=$true;installer=[IO.Path]::GetFullPath($InstallerPath);sha256=(Get-FileHash -LiteralPath $InstallerPath -Algorithm SHA256).Hash.ToLowerInvariant();productVersion=$taskProperties.ProductVersion;checks=$taskChecks;noProductInstallation=$true;interactiveRenderingObserved=$false} | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $EvidencePath -Encoding utf8
Write-Output ('PASS: native MSI extension UI and transaction tables; '+$taskChecks.Count+' checks; no installation performed')
