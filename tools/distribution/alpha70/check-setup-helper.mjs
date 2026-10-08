// Executes the real helper with process/registry seams replaced. No installer runs.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

assert.equal(process.platform, 'win32');
assert.equal(process.argv[2], '--package-dir');
assert.equal(process.argv.length, 4);
const packageDir = path.resolve(process.argv[3]);
const helper = path.join(packageDir, 'Install-Windows.ps1');
assert.equal(fs.readFileSync(helper, 'utf8'), fs.readFileSync(new URL('./Install-Windows.ps1', import.meta.url), 'utf8'));
const output = fs.mkdtempSync(path.join(os.tmpdir(), 'syndocal-alpha70-helper-tests-'));
const script = `
$ErrorActionPreference='Stop'
Import-Module Microsoft.PowerShell.Utility,Microsoft.PowerShell.Security,Microsoft.PowerShell.Management -ErrorAction Stop
function Start-Process {
 param([string]$FilePath,[string[]]$ArgumentList,[string]$WindowStyle,[switch]$Wait,[switch]$PassThru)
 if(!$Wait -or !$PassThru){throw 'Installer lifecycle must be waited and inspected'}
 $kind=if([System.IO.Path]::GetFileName($FilePath) -eq 'VC_redist.x64.exe'){'runtime'}elseif([System.IO.Path]::GetFileName($FilePath) -eq 'Syndocal_1.2.0-alpha.70_x64-setup.exe'){'app'}else{throw 'Unexpected process'}
 Add-Content -LiteralPath $env:SYNDOCAL_TEST_CALLS -Value $kind
 [pscustomobject]@{ExitCode=if($kind -eq 'runtime'){[int]$env:SYNDOCAL_TEST_RUNTIME_EXIT}else{[int]$env:SYNDOCAL_TEST_APP_EXIT}}
}
function Get-ItemProperty {
 param([string]$LiteralPath)
 if($LiteralPath -ne 'HKLM:\\SOFTWARE\\Microsoft\\VisualStudio\\14.0\\VC\\Runtimes\\x64'){throw 'Unexpected registry probe'}
 [pscustomobject]@{Installed=1;Version=$env:SYNDOCAL_TEST_RUNTIME_VERSION}
}
& $env:SYNDOCAL_TEST_HELPER
exit $LASTEXITCODE
`;
const cases = [
 ['success',0,0,'v14.51.36247.0',0,['runtime','app']],
 ['setup-cancel',0,1602,'v14.51.36247.0',1602,['runtime','app']],
 ['runtime-cancel',1602,0,'v14.51.36247.0',1,['runtime']],
 ['restart',3010,0,'v14.51.36247.0',3010,['runtime']],
 ['already-newer',1638,0,'v14.52.36247.0',0,['runtime','app']],
 ['already-too-old',1638,0,'v14.43.1.0',1,['runtime']],
];
const results=[];
for(const [name,runtimeExit,appExit,runtimeVersion,expectedExit,expectedCalls] of cases){
 const callsFile=path.join(output,name+'.txt');
 const r=spawnSync('powershell.exe',['-NoLogo','-NoProfile','-NonInteractive','-EncodedCommand',Buffer.from(script,'utf16le').toString('base64')],{windowsHide:true,encoding:'utf8',timeout:30000,env:{...process.env,PSModulePath:path.join(process.env.SystemRoot,'System32','WindowsPowerShell','v1.0','Modules'),SYNDOCAL_TEST_HELPER:helper,SYNDOCAL_TEST_CALLS:callsFile,SYNDOCAL_TEST_RUNTIME_EXIT:String(runtimeExit),SYNDOCAL_TEST_APP_EXIT:String(appExit),SYNDOCAL_TEST_RUNTIME_VERSION:runtimeVersion}});
 if(r.error)throw r.error;
 assert.equal(r.status,expectedExit,name+' '+r.stderr);
 const calls=fs.existsSync(callsFile)?fs.readFileSync(callsFile,'utf8').trim().split(/\r?\n/):[];
 assert.deepEqual(calls,expectedCalls,name);
 results.push({name,exitCode:r.status,calls,passed:true});
}
fs.writeFileSync(path.join(output,'report.json'),JSON.stringify({passed:true,cases:results,noInstallerExecuted:true,helperSource:fileURLToPath(new URL('./Install-Windows.ps1',import.meta.url))},null,2),{flag:'wx'});
console.log(JSON.stringify({passed:true,cases:results.length,evidence:path.join(output,'report.json'),noInstallerExecuted:true}));
