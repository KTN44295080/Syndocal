import fs from 'node:fs/promises';
import path from 'node:path';
import {spawn,execFile} from 'node:child_process';
import {promisify} from 'node:util';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {openNativeBackendSession} from './session.mjs';
const exec=promisify(execFile),pause=ms=>new Promise(r=>setTimeout(r,ms));
const dir=path.resolve('target/qa/task20d'),exe=path.resolve('target/release/syndocal.exe');
const [name,phase='after',secondsArg='20']=process.argv.slice(2),seconds=Number(secondsArg),tag=`${phase}-${name}`;
assert.match(tag,/^[a-z0-9-]+$/);assert.ok(seconds>=10&&seconds<=30);
const cases=JSON.parse(await fs.readFile(path.join(dir,'cases.json'),'utf8'));
const spec=cases.find(c=>c.tag===name);assert.ok(spec);
const fixture=path.join(dir,name+'.sdc');
const originalStorage=JSON.parse(await fs.readFile(path.join(dir,'original-storage.json'),'utf8'));
const report={tag,spec,at:new Date().toISOString(),samples:[],controls:[]};
report.harnessSha256=Object.fromEntries(await Promise.all(['run-case.mjs','sample-usage.ps1','session.mjs','resize-output.ps1','verify-measurement.py','policy.json'].map(async n=>[n,createHash('sha256').update(await fs.readFile(path.join(dir,n))).digest('hex')])));
report.build=JSON.parse(await fs.readFile(path.join(dir,phase+'-build.json'),'utf8'));
report.executableSha256=createHash('sha256').update(await fs.readFile(exe)).digest('hex');
assert.equal(report.executableSha256,report.build.executableSha256,'exact recorded build');

report.fixtureSha256Before=createHash('sha256').update(await fs.readFile(fixture)).digest('hex');
let b,pid,accept;
class NegativeProofComplete extends Error {}
const ps=(file,args=[],timeout=60000)=>exec('powershell.exe',['-NoProfile','-ExecutionPolicy','Bypass','-File',path.join(dir,file),...args],{windowsHide:true,timeout,maxBuffer:4*1024*1024});
async function wait(test,label,timeout=10000){const start=Date.now();while(Date.now()-start<timeout){const value=await test();if(value)return value;await pause(200)}throw Error(`${label}: timeout`)}
async function control(kind){
 const maps={acquire:['acquire_output_lease_v2','syndocal.output.lease.acquire.v2'],both:['acquire_output_lease_v2','syndocal.output.lease.acquire.v2'],relinquish:['relinquish_output_lease_v2','syndocal.output.lease.relinquish.v2'],arm:['arm_output_control_v2','syndocal.output.ownership.arm.v2'],display:['set_display_output_window_open_v2','syndocal.output.display.window.set_open.v2']};
 const [command,operation_id]=maps[kind];
 const action=kind==='acquire'||kind==='both'?{kind:'acquire_lease',role:kind==='both'?'both':'video'}:kind==='arm'?{kind:'arm',role:'video'}:kind==='relinquish'?{kind:'relinquish_output_lease'}:{kind:'set_display_window_open',output_id:1,open:true};
 if(!['acquire','both'].includes(kind)){const q=await b.invoke('query_output_lease_authority_v1');const lease=q.statuses.find(s=>s.status==='held_active'&&s.resources.includes('video'));assert.ok(lease,'active video lease');action.lease=lease.authority}
 const authority=await b.invoke('query_output_control_authority_v1'),id=await b.invoke('allocate_output_control_request_id_v1');
 const result=await b.invoke(command,{request:{operation_id,request_id:id,expected_fence:authority.fence,action}});
 report.controls.push({kind,result});console.log(JSON.stringify({tag,kind,result}));
 assert.equal(result.type,'receipt',`${kind} rejected: ${JSON.stringify(result)}`);return result;
}
try{
 const launch=spawn('powershell.exe',['-NoProfile','-File',path.join(dir,'launch.ps1'),'-Fixture',fixture,'-Tag',tag],{windowsHide:true,stdio:'ignore'});
 const launchExit=await new Promise(r=>launch.on('exit',r));assert.equal(launchExit,0,'QA launch');
 report.launch=JSON.parse(await fs.readFile(path.join(dir,tag+'-launch.json'),'utf8'));pid=report.launch.pid;
 const descriptor=path.join(process.env.LOCALAPPDATA,'jp.seraf.ktn.syndocal/agent-bridge-v1.json');
 await wait(async()=>{try{return JSON.parse(await fs.readFile(descriptor,'utf8')).processId===pid}catch{return false}},'fresh bridge descriptor',20000);
 b=await openNativeBackendSession({executable:exe,descriptor},9270);
 report.loaded=await wait(async()=>{const s=await b.invoke('get_project_authority_bundle');report.loadObservation={path:s.current_project_path,layers:s.snapshot.video.layers.length};return s.snapshot.video.layers.length===spec.layers?s:null},'QA fixture identity',15000);
 assert.ok(report.loaded.current_project_path?.endsWith(name+'.sdc'),'exact fixture path');
 assert.ok(report.loaded.snapshot.dmx_outputs.every(o=>!o.enabled),'DMX disabled');assert.equal(report.loaded.snapshot.output.enabled,false);
 if(spec.groups){assert.equal(report.loaded.snapshot.video.layer_groups.length,1);assert.equal(report.loaded.snapshot.video.layer_groups[0].opacity,0.9);assert.equal(report.loaded.snapshot.video.layer_groups[0].blend_mode,'Screen')}
 const stageCount=e=>!e||!e.enabled?0:1+(e.stack??[]).reduce((n,x)=>n+stageCount(x),0);
 report.requestedFxStages=report.loaded.snapshot.video.layers.map(l=>stageCount(l.isf_effect));
 assert.ok(report.requestedFxStages.every(n=>n===spec.fx),'actual enabled ISF stages per layer match case');
 assert.ok(report.loaded.snapshot.video.layers.every(l=>l.state.enabled&&l.state.playing),'all intended source layers are active and playing');
 report.runtime=await b.invoke('get_video_runtime_status');
 report.monitors=await b.invoke('list_video_display_monitors');
 // This process is bound to the exact release executable and only accepts its output confirmations.
 accept=spawn('powershell.exe',['-NoProfile','-File',path.join(dir,'accept-native.ps1'),'-AppProcessId',String(pid),'-Seconds','180'],{windowsHide:true,stdio:'ignore'});
 await b.evaluate(`(()=>{const key='syndocal.workspaceLayout.v1';const s=JSON.parse(localStorage.getItem(key)||'{}');Object.assign(s,{workspace_tab:'touch',control_mode:'perform',touch_control_domain:'video'});localStorage.setItem(key,JSON.stringify(s));localStorage.setItem('syndocal.uiLocale.v1','en');return true})()`);
 console.log(tag+' desk preferences set; retain live WebView context');
 await b.evaluate(`(()=>{document.querySelector('[data-workspace-option="touch"]')?.click();return true})()`);
 await pause(250);
 await b.evaluate(`(()=>{document.querySelector('[data-control-domain="video"]')?.click();return true})()`);
 await control('acquire');await control('arm');
 if(phase==='denial'){
  const lease=(await b.invoke('query_output_lease_authority_v1')).statuses.find(s=>s.status==='held_active'&&s.resources.includes('video'));
  assert.equal(lease.resources.length,1);
  const authority=await b.invoke('query_output_control_authority_v1'),id=await b.invoke('allocate_output_control_request_id_v1');
  const result=await b.invoke('set_display_output_window_open_v2',{request:{operation_id:'syndocal.output.display.window.set_open.v2',request_id:id,expected_fence:authority.fence,action:{kind:'set_display_window_open',output_id:1,open:true,lease:lease.authority}}});
  report.denial={result,windows:await b.invoke('get_video_output_window_statuses'),health:await b.invoke('get_live_video_output_health')};
  assert.equal(result.type,'rejected');assert.ok(report.denial.windows.every(w=>!w.live_open));assert.deepEqual(report.denial.health,[]);
  report.ok=true;throw new NegativeProofComplete();
 }
 for(const step of ['armed','relinquish','both']){
  if(step!=='armed')await control(step);
  const status=await b.invoke('get_output_ownership_status');
  assert.equal(status.effective_role,'Video');assert.equal(status.lighting_allowed,false);assert.equal(status.video_allowed,true);
 }
 await b.evaluate(`(()=>{const j={busy:false};j.timer=setInterval(async()=>{if(j.busy)return;j.busy=true;try{const i=window.__TAURI_INTERNALS__.invoke,q=await i('query_output_lease_authority_v1'),l=q.statuses.find(s=>s.status==='held_active'&&s.resources.includes('video'));if(!l)throw Error('video lease missing');const a=await i('query_output_control_authority_v1'),id=await i('allocate_output_control_request_id_v1');j.last=await i('renew_output_lease_v2',{request:{operation_id:'syndocal.output.lease.renew.v2',request_id:id,expected_fence:a.fence,action:{kind:'renew_lease',lease:l.authority}}});if(j.last.type!=='receipt')throw Error(JSON.stringify(j.last))}catch(e){j.error=e instanceof Error?String(e):(()=>{try{return JSON.stringify(e)??String(e)}catch{return String(e)}})();clearInterval(j.timer)}finally{j.busy=false}},2000);window.__task20renew=j;return true})()`);
 await control('display');
 await wait(async()=>{const s=await b.invoke('get_video_output_window_statuses');return s.find(o=>o.output_id===1&&o.live_open&&o.ownership_allowed)},'Display active');
 const observation=await b.invoke('get_video_output_window_observation_v1');
 const display=observation.outputs.find(o=>o.output_id==='1'&&o.live_open);
 assert.ok(display?.native_window_handle_decimal,'app-owned output handle');
 report.geometry=JSON.parse((await ps('resize-output.ps1',['-AppProcessId',String(pid),'-WindowHandle',display.native_window_handle_decimal,'-Width',spec.output==='display4k'?'3840':'1920','-Height',spec.output==='display4k'?'2160':'1080'])).stdout.trim());
 await b.invoke('stage_vj_preview_layer',{layerId:1});await b.invoke('set_vj_preview_playing',{playing:true});
 report.geometry=JSON.parse((await ps('resize-output.ps1',['-AppProcessId',String(pid),'-WindowHandle',display.native_window_handle_decimal,'-Width',spec.output==='display4k'?'3840':'1920','-Height',spec.output==='display4k'?'2160':'1080','-Present'])).stdout.trim());
 await pause(10000);
 report.startWindows=await b.invoke('get_video_output_window_statuses');
 await b.evaluate(`(()=>{const key='__task20dount',original=CanvasRenderingContext2D.prototype.drawImage,seen=new WeakMap();const state={original,start:performance.now(),preview:0,program:0};CanvasRenderingContext2D.prototype.drawImage=function(...args){const bus=this.canvas.closest('.liveVideoMonitor');if(bus&&seen.get(this.canvas)!==args[0]){seen.set(this.canvas,args[0]);state[bus.getAttribute('data-live-video-monitor')==='preview'?'preview':'program']++}return original.apply(this,args)};window[key]=state;return true})()`);
 const usage=spawn('powershell.exe',['-NoProfile','-File',path.join(dir,'sample-usage.ps1'),'-RootProcessId',String(pid),'-Seconds',String(seconds),'-Out',path.join(dir,tag+'-usage.json')],{windowsHide:true,stdio:'ignore'});
 const usageDone=new Promise(r=>usage.on('exit',r));
 const pm=spawn(path.join(process.env.LOCALAPPDATA,'SyndocalDev/presentmon/PresentMon-2.6.0-x64.exe'),['--process_id',String(pid),'--output_file',path.join(dir,tag+'-presentmon.csv'),'--timed',String(seconds),'--terminate_after_timed','--no_console_stats','--session_name','Task20-'+tag],{windowsHide:true,stdio:'pipe'});
 let pmLog='';pm.stdout.on('data',d=>pmLog+=d);pm.stderr.on('data',d=>pmLog+=d);const pmDone=new Promise(r=>pm.on('exit',r));
 const end=performance.now()+seconds*1000;
 while(performance.now()<end){report.samples.push({at:new Date().toISOString(),monotonicMs:performance.now(),health:await b.invoke('get_live_video_output_health'),windows:await b.invoke('get_video_output_window_statuses')});await pause(1000)}
 report.monitor=await b.evaluate(`(()=>{const s=window.__task20dount;CanvasRenderingContext2D.prototype.drawImage=s.original;const elapsedMs=performance.now()-s.start;delete window.__task20dount;return {elapsedMs,programFps:s.program*1000/elapsedMs,previewFps:s.preview*1000/elapsedMs}})()`);
 report.ui=await b.evaluate(`({title:document.title,width:innerWidth,height:innerHeight,health:document.querySelector('[data-live-video-output-health]')?.innerText,monitors:[...document.querySelectorAll('.liveVideoMonitor')].map(e=>e.innerText),renewError:window.__task20renew?.error??null})`);
 report.usageExit=await usageDone;report.presentExit=await pmDone;await fs.writeFile(path.join(dir,tag+'-presentmon.log'),pmLog);
 assert.equal(report.usageExit,0,'process sampler completed');assert.equal(report.presentExit,0,'PresentMon completed');
 assert.equal(report.ui.renewError,null);assert.ok(report.samples.every(s=>s.windows.some(o=>o.output_id===1&&o.live_open&&o.ownership_allowed)),'Display stayed active');
 const outputMetrics=report.samples.at(-1).windows.find(o=>o.output_id===1).performance;
 assert.ok(outputMetrics,'output metrics');assert.equal(outputMetrics.width,report.geometry.width);assert.equal(outputMetrics.height,report.geometry.height);
 assert.equal(outputMetrics.last_error,null,'output render error');
 report.compositionEvidence={rotations:report.loaded.snapshot.video.layers.map(l=>l.state.transform.rotation_deg),layerSlots:outputMetrics.layer_slots,compressedUploads:outputMetrics.compressed_layer_uploads,path:outputMetrics.layer_slots>0?(spec.groups?'CPU group isolation with GPU outer composition':'GPU composition buffers observed'):(spec.layers===1?'CPU composition: single-layer eligibility rule':'CPU composition; no GPU composition layers retained')};
 if(spec.cpu){assert.ok(report.compositionEvidence.rotations.every(v=>v!==0),'CPU fixture fails exact-zero rotation GPU eligibility');assert.equal(outputMetrics.layer_slots,0,'forced CPU case retains no GPU composition layers')}
 const decode=outputMetrics.decoder_diagnostics;
 assert.ok(decode.hap_successes+decode.libav_successes+decode.cli_fallback_successes>0,'real media decoded');
 assert.equal(decode.decode_failures,0,'a black fallback is not video performance evidence');
 for(const kind of ['display',...(spec.output==='spout'?['spout']:[]),...(spec.output==='ndi'?['ndi']:[])]){
  const samples=report.samples.flatMap(s=>s.health.filter(h=>h.kind===kind));
  assert.ok(samples.length>=2,kind+' output has real delivery samples');
  assert.ok(samples.at(-1).deliveredFrames>samples[0].deliveredFrames,kind+' output advanced during observation');
 }

 report.instrumentation=JSON.parse((await exec('python',[path.join(dir,'verify-measurement.py'),path.join(dir,tag),String(pid),String(seconds)],{windowsHide:true,timeout:15000,maxBuffer:2*1024*1024})).stdout);

 if(phase.startsWith('probe') || (phase==='after' && spec.layers===3 && spec.source==='h264_1080p30.mp4' && ['display','spout'].includes(spec.output) && [0,3].includes(spec.fx) && spec.blend==='Normal' && !spec.cpu && !spec.groups)){
  report.nativeDisplayCapture=JSON.parse((await ps('capture-native-display.ps1',['-AppProcessId',String(pid),'-WindowHandle',display.native_window_handle_decimal,'-Out',path.join(dir,tag+'-native-display.png')])).stdout.trim());
  assert.ok(report.nativeDisplayCapture.nonBlackSampleFraction>0.01,'native Display screenshot contains the QA media');
  report.localeProof=[];
  for(const locale of ['en','ja']){
   await b.evaluate(`(()=>{localStorage.setItem('syndocal.uiLocale.v1','${locale}');window.dispatchEvent(new StorageEvent('storage',{key:'syndocal.uiLocale.v1',storageArea:localStorage}));return true})()`);await pause(750);
   const proof=await b.evaluate(`(()=>{const e=document.querySelector('[data-live-video-output-health]');if(!e)return null;const r=e.getBoundingClientRect();return {text:e.innerText,title:e.title,bounds:{x:r.x,y:r.y,width:r.width,height:r.height},viewport:{width:innerWidth,height:innerHeight},leaves:[...e.querySelectorAll('span')].filter(n=>!n.children.length).map(n=>({text:n.innerText,fontPx:parseFloat(getComputedStyle(n).fontSize),color:getComputedStyle(n).color,bounds:{x:n.getBoundingClientRect().x,y:n.getBoundingClientRect().y,width:n.getBoundingClientRect().width,height:n.getBoundingClientRect().height}}))}})()`);
   assert.ok(proof?.text?.length>0,'native performance readout is rendered');report.localeProof.push({locale,...proof});
   const capture=await b.send('Page.captureScreenshot',{format:'png',captureBeyondViewport:false});await fs.writeFile(path.join(dir,tag+'-'+locale+'.png'),Buffer.from(capture.data,'base64'));
  }
 }
 report.ok=true;
}catch(e){if(!(e instanceof NegativeProofComplete)){report.error=String(e);console.error(report.error);if(b)try{report.errorUi=await b.evaluate('document.body.innerText.slice(0,8000)')}catch{};process.exitCode=1}}
finally{
 let restoreAttemptError;
 if(b){try{
  await b.evaluate(`(()=>{if(window.__task20renew)clearInterval(window.__task20renew.timer);const s=window.__task20dount;if(s){CanvasRenderingContext2D.prototype.drawImage=s.original;delete window.__task20dount}return true})()`);
  report.restoredStorage=await b.evaluate(`(()=>{localStorage.clear();for(const [k,v] of Object.entries(${JSON.stringify(originalStorage)}))localStorage.setItem(k,v);return Object.fromEntries(Object.entries(localStorage))})()`);
  assert.deepEqual(report.restoredStorage,originalStorage);
  await b.invoke('set_output_ownership_role',{role:'Standby'});
  report.restoredRole=await b.invoke('get_output_ownership_status');report.restoredWindows=await b.invoke('get_video_output_window_statuses');
  assert.equal(report.restoredRole.role,'Standby');assert.ok(report.restoredWindows.every(w=>!w.live_open&&!w.test_pattern_open));
 }catch(e){restoreAttemptError=String(e);report.restoreWarnings=[restoreAttemptError];console.error('RESTORE RETRY AFTER EXACT APP SHUTDOWN',e)}finally{b.close()}}
 if(accept)accept.kill();
 if(pid)try{report.stop=(await ps('stop.ps1',['-AppProcessId',String(pid)])).stdout.trim()}catch(e){report.stopError=String(e);process.exitCode=1}
 if(restoreAttemptError){
  if(report.stopError){report.restoreError=restoreAttemptError;process.exitCode=1}
  else try{
   await exec(process.execPath,[path.join(dir,'restore-state.mjs'),tag+'-recovery'],{windowsHide:true,timeout:180000,maxBuffer:2*1024*1024});
   report.cleanLaunchRestore=JSON.parse(await fs.readFile(path.join(dir,tag+'-recovery.json'),'utf8'));
   assert.equal(report.cleanLaunchRestore.ok,true);
  }catch(e){report.restoreError=String(e);process.exitCode=1}
 }

 report.fixtureSha256After=createHash('sha256').update(await fs.readFile(fixture)).digest('hex');
 assert.equal(report.fixtureSha256After,report.fixtureSha256Before,'QA source show not overwritten');
 await fs.writeFile(path.join(dir,tag+'.json'),JSON.stringify(report,null,2));
 console.log(JSON.stringify({tag,ok:report.ok,error:report.error,restoreError:report.restoreError,health:report.samples.at(-1)?.health,monitor:report.monitor}));
}
