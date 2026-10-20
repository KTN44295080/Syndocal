import fs from 'node:fs/promises';
import path from 'node:path';
import {spawn,execFile} from 'node:child_process';
import {promisify} from 'node:util';
const exec=promisify(execFile);
import assert from 'node:assert/strict';
const dir=path.resolve('target/qa/task20d');
const args=process.argv.slice(2),selected=[];
let phase='after',seconds=20;
for(let i=0;i<args.length;i++){
 if(args[i]==='--phase')phase=args[++i];
 else if(args[i]==='--seconds')seconds=Number(args[++i]);
 else selected.push(args[i]);
}
assert.ok(['before','after'].includes(phase));
assert.ok(seconds>=10&&seconds<=30);
const cases=JSON.parse(await fs.readFile(path.join(dir,'cases.json'),'utf8'));
const build=JSON.parse(await fs.readFile(path.join(dir,phase+'-build.json'),'utf8'));
const results=[],guardStates=[];
let failures=0;
for(const c of cases){
 if(selected.length&&!selected.includes(c.tag))continue;
 if(c.output==='ndi'&&!build.ndiEnabled){
  results.push({case:c.tag,status:'unsupported',reason:'Required default release has NDI NotBuilt; no sender enabled.'});
  continue;
 }
 const reportPath=path.join(dir,phase+'-'+c.tag+'.json');
 try{
  const old=JSON.parse(await fs.readFile(reportPath,'utf8'));
  assert.equal(old.executableSha256,build.executableSha256,'Do not overwrite another build evidence');
  if(old.ok&&!old.restoreError&&!old.stopError){results.push({case:c.tag,status:'measured',reused:true});continue}
  if(old.error==='Error: Native backend deadline'&&!old.restoreError&&!old.stopError){results.push({case:c.tag,status:'workload_failed',reason:'native_backend_startup_deadline',reused:true});continue}
 }catch(e){if(e.code!=='ENOENT')throw e}
 const guardStart=Date.now();
 while(true){
  const guard=JSON.parse((await exec('powershell.exe',['-NoProfile','-File',path.join(dir,'get-build-state.ps1')],{windowsHide:true,timeout:15000})).stdout);
  guardStates.push({case:c.tag,...guard});
  await fs.writeFile(path.join(dir,phase+'-build-isolation.json'),JSON.stringify(guardStates,null,2));
  if(guard.idle)break;
  assert.ok(Date.now()-guardStart<1800000,'Concurrent build did not finish');
  console.log('WAIT for concurrent build '+JSON.stringify(guard.processes));
  await new Promise(r=>setTimeout(r,5000));
 }
 console.log('START '+new Date().toISOString()+' '+phase+' '+c.tag);
 const output=await fs.open(path.join(dir,phase+'-'+c.tag+'-runner.log'),'wx');
 const child=spawn(process.execPath,[path.join(dir,'run-case.mjs'),c.tag,phase,String(seconds)],{
  windowsHide:true,stdio:['ignore',output.fd,output.fd]
 });
 const code=await new Promise((resolve,reject)=>{child.on('error',reject);child.on('exit',resolve)});
 await output.close();
 const result={case:c.tag,exit:code,status:code===0?'measured':'failed',at:new Date().toISOString()};
 results.push(result);
 console.log('END '+JSON.stringify(result));
 await fs.writeFile(path.join(dir,phase+'-batch-progress.json'),JSON.stringify(results,null,2));
 const report=JSON.parse(await fs.readFile(reportPath,'utf8'));
 if(report.restoreError||report.stopError){process.exitCode=1;console.log('STOP: restoration must be resolved before another case');break}
 const overlap=String(report.error||'').includes('concurrent compiler or test workload invalidates performance comparison');
 const startup=report.error==='Error: Native backend deadline';
 failures=code===0||overlap||startup?0:failures+1;
 if(failures>=3){process.exitCode=1;console.log('STOP: three consecutive case failures');break}
}
await fs.writeFile(path.join(dir,phase+'-batch-progress.json'),JSON.stringify(results,null,2));
