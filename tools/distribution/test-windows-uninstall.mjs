// Execute the real uninstall orchestrator with disposable registrations and
// process/cleanup seams. No installer or user profile is touched.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import test from 'node:test';
import { spawnSync } from 'node:child_process';
const source = fs.readFileSync(new URL('./Uninstall-Windows.ps1',import.meta.url),'utf8');
const guid='{12345678-1234-1234-1234-123456789abc}';
for(const scenario of ['msi','nsis','cancel','unknown','validate']) test(`uninstall orchestration ${scenario}`,{skip:process.platform!=='win32'},()=>{
  const root=fs.mkdtempSync(path.join(os.tmpdir(),'syndocal-uninstall-test-'));
  const helper=path.join(root,'Uninstall-Windows.ps1'),installed=path.join(root,'installed');fs.mkdirSync(installed);
  fs.writeFileSync(path.join(installed,'syndocal.exe'),'fixture');fs.writeFileSync(path.join(installed,'uninstall.exe'),'fixture');
  fs.writeFileSync(helper,source);fs.writeFileSync(path.join(root,'Remove-SyndocalData.ps1'),"Set-Content -LiteralPath $env:SYNDOCAL_TEST_PURGED -Value purged\n");
  fs.copyFileSync(new URL('./SyndocalInstallations.ps1',import.meta.url),path.join(root,'SyndocalInstallations.ps1'));
  const script=`
function Get-Process { param($Name,$ErrorAction) @() }
function Get-ChildItem { param($LiteralPath,$ErrorAction)
 if($LiteralPath.StartsWith('HKCU:')){[pscustomobject]@{PSPath='fixture-registry'}}
}
function Get-ItemProperty {param($LiteralPath)
 if($LiteralPath -ne 'fixture-registry'){throw 'Unexpected registration'}
 $uninstall=if($env:SYNDOCAL_TEST_KIND -eq 'nsis'){ '\"'+(Join-Path $env:SYNDOCAL_TEST_INSTALL 'uninstall.exe')+'\"' }
 elseif($env:SYNDOCAL_TEST_KIND -eq 'unknown'){ 'unexpected.exe' }else{ 'MsiExec.exe /I${guid}' }
 [pscustomobject]@{DisplayName='Syndocal';Publisher='Seraf() / KTN';UninstallString=$uninstall;InstallLocation=$env:SYNDOCAL_TEST_INSTALL}
}
function Read-Host {param($Prompt) 'REMOVE'}
function Start-Process {param($FilePath,$ArgumentList,$WindowStyle,$Wait,$PassThru)
 if(!$Wait -or !$PassThru){throw 'Uninstaller result must be inspected'}
 if($env:SYNDOCAL_TEST_KIND -ne 'nsis' -and ($ArgumentList -join ' ') -ne '/x ${guid}'){throw 'MSI uninstall must use /x'}
 Set-Content -LiteralPath $env:SYNDOCAL_TEST_CALLED -Value called
 [pscustomobject]@{ExitCode=if($env:SYNDOCAL_TEST_KIND -eq 'cancel'){1602}else{0}}
}
& $env:SYNDOCAL_TEST_HELPER ${scenario==='validate'?'-ValidateOnly':''}
exit $LASTEXITCODE
`;
  const called=path.join(root,'called'),purged=path.join(root,'purged');
  const r=spawnSync('powershell.exe',['-NoProfile','-NonInteractive','-EncodedCommand',Buffer.from(script,'utf16le').toString('base64')],{
    encoding:'utf8',windowsHide:true,timeout:10000,env:{...process.env,SYNDOCAL_TEST_KIND:scenario,SYNDOCAL_TEST_HELPER:helper,
      SYNDOCAL_TEST_INSTALL:installed,SYNDOCAL_TEST_CALLED:called,SYNDOCAL_TEST_PURGED:purged,
      PSModulePath:path.join(process.env.SystemRoot,'System32/WindowsPowerShell/v1.0/Modules')}});
  assert.equal(r.status,scenario==='cancel'||scenario==='unknown'?1:0,r.stderr);
  assert.equal(fs.existsSync(purged),scenario==='msi'||scenario==='nsis');
  assert.equal(fs.existsSync(called),['msi','nsis','cancel'].includes(scenario));
  fs.rmSync(root,{recursive:true});
});
