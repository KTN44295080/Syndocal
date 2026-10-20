import fs from 'node:fs/promises';
import path from 'node:path';
import {execFile} from 'node:child_process';
import {promisify} from 'node:util';
import assert from 'node:assert/strict';
import {openNativeBackendSession} from './session.mjs';
const exec=promisify(execFile),dir=path.resolve('target/qa/task20d');
const mode=process.argv[2]??'original';
assert.ok(['original','verify-restored'].includes(mode));
const ps=(file,args=[])=>exec('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File',path.join(dir,file),...args],{windowsHide:true,timeout:60000});
let b,pid,storage;
try{
 if(mode==='original')assert.equal(await fs.stat(path.join(dir,'original-storage.json')).then(()=>true,()=>false),false,'Never overwrite original preferences');
 await ps('launch-blank.ps1',['-Tag',mode]);
 const launch=JSON.parse(await fs.readFile(path.join(dir,mode+'-launch.json'),'utf8'));pid=launch.pid;
 const descriptor=path.join(process.env.LOCALAPPDATA,'jp.seraf.ktn.syndocal/agent-bridge-v1.json');
 let fresh=false;
 for(let n=0;n<80;n++){try{fresh=JSON.parse(await fs.readFile(descriptor,'utf8')).processId===pid}catch{}if(fresh)break;await new Promise(r=>setTimeout(r,250))}
 assert.ok(fresh,'fresh descriptor');
 b=await openNativeBackendSession({executable:path.resolve('target/release/syndocal.exe'),descriptor},9270);
 storage=await b.evaluate('Object.fromEntries(Object.entries(localStorage))');
 const role=await b.invoke('get_output_ownership_status'),windows=await b.invoke('get_video_output_window_statuses');
 assert.equal(role.role,'Standby','Start from Standby');
 assert.ok(windows.every(w=>!w.live_open),'Start with outputs off');
 if(mode==='original'){
  await fs.writeFile(path.join(dir,'original-storage.json'),JSON.stringify(storage,null,2),{flag:'wx'});
  await fs.writeFile(path.join(dir,'original-authority.json'),JSON.stringify(await b.invoke('get_project_authority_bundle'),null,2),{flag:'wx'});
  const monitors=await b.invoke('list_video_display_monitors'),runtime=await b.invoke('get_video_runtime_status');
  await fs.writeFile(path.join(dir,'preflight.json'),JSON.stringify({launch,role,windows,monitors,runtime},null,2));
  const primary=monitors.filter(m=>m.primary);assert.equal(primary.length,1);assert.equal(monitors.length,1);
  for(const c of JSON.parse(await fs.readFile(path.join(dir,'cases.json'),'utf8'))){
   const file=path.join(dir,c.tag+'.sdc'),fixture=JSON.parse(await fs.readFile(file,'utf8'));
   for(const o of fixture.snapshot.video.outputs)if(o.kind==='Display')Object.assign(o,{monitor_id:primary[0].index,monitor_identity:primary[0].identity,width:primary[0].physicalWidth,height:primary[0].physicalHeight,fullscreen:false});
   await fs.writeFile(file,JSON.stringify(fixture));
  }
  console.log(JSON.stringify({captured:true,preferenceKeys:Object.keys(storage).length,role:role.role,monitors,runtime}));
 }else{
  const original=JSON.parse(await fs.readFile(path.join(dir,'original-storage.json'),'utf8'));
  assert.deepEqual(storage,original,'Preferences and Recent survived shutdown and relaunch');
  await fs.writeFile(path.join(dir,'restore-proof.json'),JSON.stringify({at:new Date().toISOString(),launch,role,windows,localStorageExact:true,recentExact:storage['syndocal.recentProjects.v1']===original['syndocal.recentProjects.v1']},null,2));
  console.log(JSON.stringify({verified:true,role:role.role,localStorageExact:true,recentExact:true,outputsOff:true}));
 }
}finally{
 if(b)b.close();
 if(pid)await ps('stop.ps1',['-AppProcessId',String(pid)]);
}
