param([string]$Fixture,[string]$Tag)
$ErrorActionPreference='Stop'
$exe=(Resolve-Path target/release/syndocal.exe).Path
$qa=(Resolve-Path target/qa/task20d).Path
$fixturePath=(Resolve-Path -LiteralPath $Fixture).Path
if(!$fixturePath.StartsWith($qa+'\') -or [IO.Path]::GetExtension($fixturePath) -ne '.sdc'){throw 'QA fixture boundary'}
if(@(Get-CimInstance Win32_Process | Where-Object ExecutablePath -eq $exe).Count){throw 'Exact exe already running'}
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS='--remote-debugging-address=127.0.0.1 --remote-debugging-port=9270'
$p=Start-Process -FilePath $exe -ArgumentList ('"'+$fixturePath+'"') -WindowStyle Hidden -PassThru -RedirectStandardError "$qa/$Tag-stderr.log" -RedirectStandardOutput "$qa/$Tag-stdout.log"
$p.Id | Set-Content "$qa/pid.txt"
$deadline=(Get-Date).AddSeconds(50)
do { Start-Sleep -Milliseconds 300; $p.Refresh(); if($p.HasExited){throw 'App exited'} } while(($p.MainWindowTitle -ne 'Syndocal' -or !$p.Responding) -and (Get-Date) -lt $deadline)
if(!$p.Responding -or $p.MainWindowTitle -ne 'Syndocal'){throw 'Native identity gate failed'}
Add-Type @"
using System;using System.Runtime.InteropServices;
public static class PerfWin{[DllImport("user32.dll")]public static extern bool ShowWindow(IntPtr h,int n);[DllImport("user32.dll")]public static extern bool SetForegroundWindow(IntPtr h);}
"@
[void][PerfWin]::ShowWindow($p.MainWindowHandle,3);[void][PerfWin]::SetForegroundWindow($p.MainWindowHandle)
$result=[pscustomobject]@{pid=$p.Id;exe=$exe;responsive=$p.Responding;title=$p.MainWindowTitle;fixture=$fixturePath} | ConvertTo-Json -Compress
[IO.File]::WriteAllText("$qa/$Tag-launch.json",$result,[Text.UTF8Encoding]::new($false))
Write-Output $result
