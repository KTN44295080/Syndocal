param([int]$AppProcessId)
$ErrorActionPreference='Stop';$exe=(Resolve-Path target/release/syndocal.exe).Path
$p=Get-Process -Id $AppProcessId -ErrorAction SilentlyContinue
if($p){if($p.Path -ne $exe){throw 'Wrong process'};[void]$p.CloseMainWindow();if(!$p.WaitForExit(6000)){if((Get-Process -Id $AppProcessId).Path -ne $exe){throw 'Process identity changed'};Stop-Process -Id $AppProcessId -Force;if(!$p.WaitForExit(10000)){throw 'Exact app did not terminate after force'}}}
if(@(Get-CimInstance Win32_Process -Filter "Name = 'syndocal.exe'" | Where-Object ExecutablePath -eq $exe).Count){throw 'Exact app still running'}
