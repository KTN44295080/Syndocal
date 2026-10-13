import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import test from 'node:test';
import { spawnSync } from 'node:child_process';
const source = fs.readFileSync(new URL('./Remove-Syndocal.command',import.meta.url),'utf8');
function fixture() {
  const root = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(),'syndocal-mac-cleanup-test-')));
  const home = path.join(root,'home'), tmp = path.join(root,'T');
  const app = path.join(root,'Applications/Syndocal.app');
  const data = path.join(home,'Library/Application Support/jp.seraf.ktn.syndocal');
  for(const directory of [tmp,data,path.join(app,'Contents')]) fs.mkdirSync(directory,{recursive:true});
  fs.writeFileSync(path.join(data,'settings'),'settings');
  fs.writeFileSync(path.join(home,'keep.sdc'),'show');
  fs.writeFileSync(path.join(app,'Contents/Info.plist'),'<?xml version="1.0"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>jp.seraf.ktn.syndocal</string></dict></plist>');
  const script = path.join(root,'remove.command');
  const rewritten = source.replace('pgrep -x syndocal','false').replaceAll('/Applications/Syndocal.app',`${root}/Applications/Syndocal.app`)
    .replace('temporary=$(getconf DARWIN_USER_TEMP_DIR)',`temporary='${tmp}/'`).replace('/var/folders/*/T/',`${tmp}/`)
    .replace('/Library/Application Support/$identifier',`${root}/system/$identifier`)
    .replace(/system_receipts=\([^\n]+\)/,`system_receipts=("jp.seraf.ktn.syndocal.qa.${path.basename(root)}")`)
    .replaceAll('/usr/bin/sudo /bin/rm','/bin/rm');
  fs.writeFileSync(script,rewritten);
  return {root,home,tmp,app,data,script};
}
function run(f,args=[],input='REMOVE\n') {return spawnSync('/bin/bash',[f.script,...args],{encoding:'utf8',input,env:{...process.env,HOME:f.home},timeout:10000});}
test('Mac complete removal removes validated bundle/settings/temp and preserves external projects', {skip:process.platform!=='darwin'},()=>{
  const f=fixture(); const temp=path.join(f.tmp,'syndocal-gdtf-share-123-456.cookies');fs.writeFileSync(temp,'cookie');
  const r=run(f);assert.equal(r.status,0,r.stderr);assert.ok(!fs.existsSync(f.app));assert.ok(!fs.existsSync(f.data));assert.ok(!fs.existsSync(temp));
  assert.equal(fs.readFileSync(path.join(f.home,'keep.sdc'),'utf8'),'show');fs.rmSync(f.root,{recursive:true});
});
test('Mac validation and cancellation preserve all files', {skip:process.platform!=='darwin'},()=>{
  const f=fixture();assert.equal(run(f,['--validate-only']).status,0);assert.equal(run(f,[],'NO\n').status,0);
  assert.ok(fs.existsSync(f.app));assert.ok(fs.existsSync(f.data));fs.rmSync(f.root,{recursive:true});
});
test('Mac redirected data target rejects removal before any bundle deletion', {skip:process.platform!=='darwin'},()=>{
  const f=fixture();fs.rmSync(f.data,{recursive:true});fs.symlinkSync(f.home,f.data);
  const r=run(f);assert.notEqual(r.status,0);assert.match(r.stdout,/Redirected cleanup path/);
  assert.ok(fs.existsSync(f.app));assert.ok(fs.existsSync(path.join(f.home,'keep.sdc')));fs.unlinkSync(f.data);fs.rmSync(f.root,{recursive:true});
});
