import fs from 'node:fs/promises';
import path from 'node:path';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import assert from 'node:assert/strict';
import {openNativeBackendSession} from './session.mjs';
const exec=promisify(execFile),dir=path.resolve('target/qa/task20d');
const tag=process.argv[2]??'restore-recovery';
assert.match(tag,/^[a-z0-9-]+$/);
const original=JSON.parse(await fs.readFile(path.join(dir,'original-storage.json'),'utf8'));
const ps=(file,args=[])=>exec('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File',path.join(dir,file),...args],{windowsHide:true,timeout:60000});
const proof={at:new Date().toISOString(),tag,passes:[]};
for(const verifyOnly of [false,true]){
 let b,pid;
 try{
  const launchTag=tag+(verifyOnly?'-verify':'-restore');
  await ps('launch-blank.ps1',['-Tag',launchTag]);
  const launch=JSON.parse(await fs.readFile(path.join(dir,launchTag+'-launch.json'),'utf8'));pid=launch.pid;
  const descriptor=path.join(process.env.LOCALAPPDATA,'jp.seraf.ktn.syndocal/agent-bridge-v1.json');
  let fresh=false;
  for(let n=0;n<80;n++){try{fresh=JSON.parse(await fs.readFile(descriptor,'utf8')).processId===pid}catch{}if(fresh)break;await new Promise(r=>setTimeout(r,250))}
  assert.ok(fresh,'fresh clean-launch descriptor');
  b=await openNativeBackendSession({executable:path.resolve('target/release/syndocal.exe'),descriptor},9270);
  const role=await b.invoke('get_output_ownership_status'),windows=await b.invoke('get_video_output_window_statuses');
  assert.equal(role.role,'Standby');assert.ok(windows.every(w=>!w.live_open&&!w.test_pattern_open));
  if(!verifyOnly)await b.evaluate(`(()=>{localStorage.clear();for(const [k,v] of Object.entries(${JSON.stringify(original)}))localStorage.setItem(k,v);return true})()`);
  const storage=await b.evaluate('Object.fromEntries(Object.entries(localStorage))');
  assert.deepEqual(storage,original,'Exact preferences and Recent');
  proof.passes.push({verifyOnly,pid,role:role.role,outputsOff:true,localStorageExact:true,recentExact:storage['syndocal.recentProjects.v1']===original['syndocal.recentProjects.v1']});
 }finally{if(b)b.close();if(pid)await ps('stop.ps1',['-AppProcessId',String(pid)])}
}
proof.ok=true;
await fs.writeFile(path.join(dir,tag+'.json'),JSON.stringify(proof,null,2));
console.log(JSON.stringify(proof));
