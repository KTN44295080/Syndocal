// Run the real filesystem policy against disposable roots. OS vault/process
// seams are replaced, so a running user application and its credentials survive.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import test from 'node:test';
import { spawnSync } from 'node:child_process';
const original = fs.readFileSync(new URL('./Remove-SyndocalData.ps1', import.meta.url), 'utf8');
const vaultStart = original.indexOf('# OS credentials');
const removalStart = original.indexOf('foreach ($taskTarget in $taskTargets)', vaultStart);
assert.ok(vaultStart > 0 && removalStart > vaultStart);
const source = original.slice(0, vaultStart) + original.slice(removalStart);
function fixture() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'syndocal-cleanup-test-'));
  const env = { ...process.env, APPDATA:path.join(root,'roaming'), LOCALAPPDATA:path.join(root,'local'), TEMP:path.join(root,'tmp') };
  for (const name of ['APPDATA','LOCALAPPDATA','TEMP']) fs.mkdirSync(env[name]);
  const owned = path.join(env.LOCALAPPDATA,'jp.seraf.ktn.syndocal'); fs.mkdirSync(owned);
  fs.writeFileSync(path.join(owned,'settings.json'),'settings');
  const foreign = path.join(env.LOCALAPPDATA,'another-app'); fs.mkdirSync(foreign);
  fs.writeFileSync(path.join(foreign,'keep.sdc'),'project');
  const helper = path.join(root,'cleanup.ps1'); fs.writeFileSync(helper,source);
  return {root,env,owned,foreign,helper};
}
function run(f, validate=false) {
  const script = `function Get-Process { param($Name,$ErrorAction) @() }; & $env:SYNDOCAL_CLEANUP_HELPER ${validate?'-ValidateOnly':''}; exit $LASTEXITCODE`;
  return spawnSync('powershell.exe',['-NoProfile','-NonInteractive','-EncodedCommand',Buffer.from(script,'utf16le').toString('base64')],{
    encoding:'utf8',windowsHide:true,timeout:30000,env:{...f.env,SYNDOCAL_CLEANUP_HELPER:f.helper,
      PSModulePath:path.join(process.env.SystemRoot,'System32/WindowsPowerShell/v1.0/Modules')}});
}
test('purge removes exact settings/cache/temp paths and preserves foreign projects', {skip:process.platform!=='win32'}, () => {
  const f=fixture();
  const ownedTemp=path.join(f.env.TEMP,'syndocal-media-snapshot-123-aabb-1');fs.mkdirSync(ownedTemp);fs.writeFileSync(path.join(ownedTemp,'media'),'x');
  const foreignTemp=path.join(f.env.TEMP,'syndocal-other-project.sdc');fs.writeFileSync(foreignTemp,'keep');
  const r=run(f);assert.equal(r.status,0,r.stderr);
  assert.ok(!fs.existsSync(f.owned));assert.ok(!fs.existsSync(ownedTemp));
  assert.equal(fs.readFileSync(path.join(f.foreign,'keep.sdc'),'utf8'),'project');assert.ok(fs.existsSync(foreignTemp));
  fs.rmSync(f.root,{recursive:true});
});
test('validation is non-destructive', {skip:process.platform!=='win32'}, () => {
  const f=fixture(); const r=run(f,true);assert.equal(r.status,0,r.stderr);assert.ok(fs.existsSync(f.owned));fs.rmSync(f.root,{recursive:true});
});
for (const placement of ['target','child','ancestor']) test(`junction in ${placement} rejects ALL deletions`, {skip:process.platform!=='win32'}, () => {
  const f=fixture();let link;
  if(placement==='target') {fs.rmSync(f.owned,{recursive:true});link=f.owned;}
  else if(placement==='child') link=path.join(f.owned,'redirect');
  else {link=path.join(f.root,'redirect');f.env.APPDATA=path.join(link,'roaming');fs.mkdirSync(path.join(f.foreign,'roaming'));}
  fs.symlinkSync(f.foreign,link,'junction');
  const r=run(f);assert.notEqual(r.status,0);assert.match(r.stderr,/Redirected cleanup/);
  assert.equal(fs.readFileSync(path.join(f.foreign,'keep.sdc'),'utf8'),'project');
  if(placement!=='target') assert.ok(fs.existsSync(path.join(f.owned,'settings.json')));
  fs.rmSync(link);fs.rmSync(f.root,{recursive:true});
});

test('full removal retires primary, rollback and exact generic agent credentials only', {skip:process.platform!=='win32'}, () => {
  const f=fixture();
  const fakeVault = `Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class SyndocalUninstallCredentials {
 [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Unicode)]
 public struct Credential { public uint Flags; public uint Type; public string TargetName; }
 public static bool Enumerate(string filter, uint flags, out uint count, out IntPtr credentials) {
  count=4; credentials=Marshal.AllocHGlobal(4*IntPtr.Size);
  string[] names={"Syndocal/AgentAuthority/v1/"+new string('a',64),"Syndocal/AgentAuthority/v1/foreign","foreign-app","Syndocal/AgentAuthority/v1/"+new string('b',64)};
  for(int i=0;i<4;i++) { IntPtr p=Marshal.AllocHGlobal(Marshal.SizeOf(typeof(Credential))); Marshal.StructureToPtr(new Credential {Type=(uint)(i==3?2:1),TargetName=names[i]},p,false); Marshal.WriteIntPtr(credentials,i*IntPtr.Size,p); }
  return true;
 }
 public static bool Delete(string target, uint type, uint flags) { Console.WriteLine("RemovedCredential:"+target); return true; }
 public static void Free(IntPtr buffer) { for(int i=0;i<4;i++) { IntPtr p=Marshal.ReadIntPtr(buffer,i*IntPtr.Size); Marshal.DestroyStructure(p,typeof(Credential)); Marshal.FreeHGlobal(p); } Marshal.FreeHGlobal(buffer); }
}
'@`;
  fs.writeFileSync(f.helper, original.replace(/Add-Type -TypeDefinition @'[\s\S]*?'@/,fakeVault));
  const r=run(f); assert.equal(r.status,0,r.stderr);
  const deleted=r.stdout.split(/\r?\n/).filter(line=>line.startsWith('RemovedCredential:')).map(line=>line.slice('RemovedCredential:'.length));
  assert.deepEqual(deleted.sort(),['jp.seraf.ktn.syndocal/dj-link/v1','jp.seraf.ktn.syndocal/dj-link/v1/rollback','Syndocal/AgentAuthority/v1/'+ 'a'.repeat(64)].sort());
  assert.ok(!fs.existsSync(f.owned)); fs.rmSync(f.root,{recursive:true});
});
