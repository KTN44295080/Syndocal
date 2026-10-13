import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
assert.equal(process.platform,'win32');
const root=fs.mkdtempSync(path.join(os.tmpdir(),'syndocal-installer-choice-test-'));
const script=String.raw`
$ErrorActionPreference='Stop'
. $env:SYNDOCAL_CHOICE_SOURCE
$taskPath=Get-SyndocalExtensionSettingsPath
Save-SyndocalExtensionChoice $true
if((Get-Content -LiteralPath $taskPath -Raw) -cne "[SyndocalExtensions]
Schema=1
DJLink=1
"){throw 'Choice did not save'}
Save-SyndocalExtensionChoice $false
if((Get-Content -LiteralPath $taskPath -Raw) -notmatch 'DJLink=0'){throw 'Choice did not change'}
$taskFuture="[SyndocalExtensions]
Schema=2
DJLink=1
"
[IO.File]::WriteAllText($taskPath,$taskFuture)
$taskRejected=$false
try {Save-SyndocalExtensionChoice $false}catch{$taskRejected=$true}
if(!$taskRejected -or [IO.File]::ReadAllText($taskPath) -cne $taskFuture){throw 'Future settings were replaced'}
[IO.File]::WriteAllText($taskPath, ('x'*4097))
$taskRejected=$false
try {Save-SyndocalExtensionChoice $false}catch{$taskRejected=$true}
if(!$taskRejected -or [IO.File]::ReadAllText($taskPath).Length -ne 4097){throw 'Oversized settings were replaced'}
Write-Output 'PASS: installation choice enable/disable, future schema and size rejection; no UI or installer executed'
`;
const r=spawnSync('powershell.exe',['-NoProfile','-NonInteractive','-EncodedCommand',Buffer.from(script,'utf16le').toString('base64')],{windowsHide:true,encoding:'utf8',env:{...process.env,LOCALAPPDATA:root,SYNDOCAL_CHOICE_SOURCE:fileURLToPath(new URL('./Select-SyndocalExtensions.ps1',import.meta.url))},timeout:30000});
assert.equal(r.status,0,r.stdout+'\n'+r.stderr);console.log(r.stdout.trim());
