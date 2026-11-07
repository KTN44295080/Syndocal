// Browser fixture proof for Sync disclosure. Production components and styles,
// in-memory statuses only: no native app, physical devices or network outputs.
import assert from 'node:assert/strict';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { createRequire } from 'node:module';
import { homedir } from 'node:os';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';
const require = createRequire(import.meta.url);
let chromium;
try { ({chromium}=require('playwright')); } catch { ({chromium}=require(process.env.PLAYWRIGHT_MODULE_PATH ?? resolve(homedir(),'.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright'))); }
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..');
const artifacts=resolve(root,'../target/qa/sync-disclosure-20261107');
await mkdir(artifacts,{recursive:true});
const fixtureId='/src/__sync-disclosure-fixture.tsx';
const fixture=String.raw`
import {render} from 'solid-js/web';
import {createSignal} from 'solid-js';
import {StandbySyncPanel} from '/src/components/StandbySyncPanel.tsx';
import {ShowClockStatusPanel} from '/src/components/ShowClockStatusPanel.tsx';
import {LtcInputPanel} from '/src/components/LtcInputPanel.tsx';
import {VideoOutputActionsPanel} from '/src/components/VideoOutputActionsPanel.tsx';
import {installUiLocalization} from '/src/uiLocalization.ts';
import '/src/theme.css'; import '/src/styles.css'; import '/src/setupIo.css'; import '/src/components/SyncPanels.css';
const [locale,setLocale]=createSignal('en');
let mode='stopped'; let expiredReleased=false; const log=[];
const expiryFence={process_incarnation:1,session_incarnation:2,project_epoch:3,project_revision:4,project_checkpoint_hash:'a'.repeat(64),project_publication_generation:5,output_epoch:6,output_generation:7,safety_blackout_epoch:8,safety_blackout_generation:9};
let ownership={role:'Both',effective_role:'Both',desired_role:'Both',persisted_role:'Both',state:'Ready',generation:1,epoch:1,lighting_allowed:true,video_allowed:true,lighting_reason:'OwnedByMachineRole',video_reason:'OwnedByMachineRole',error:null};
const invoke=async(command,args)=>{
 log.push({command,args});
 const active=mode!=='stopped'; const fail=mode==='failure';
 if(command==='standby_sync_status') return {running:active,role:active?'standby':null,directory:active?'C:/shared':null,session_id:null,generation:active?3:null,written_at_unix_ms:null,heartbeat_age_ms:active?25:null,heartbeat_stale:fail,takeover_ready:!fail,split_brain:fail,active_primary_sessions:fail?['a','b']:[],project_bytes:active?2400:0,last_applied_generation:active?3:null,last_error:fail?'Shared folder is unavailable':null};
 if(command==='get_output_ownership_status') return {...ownership};
 if(command==='query_output_lease_authority_v1') return {operation_id:'syndocal.output.lease.authority.query.v1',statuses:mode==='expired'?[{status:'unavailable'}]:[{status:'held_active',authority:{lease_id:'lease-0000000000000001',generation:1},resources:['lighting','video']}]};
 if(command==='query_video_authoring_lease_authority_v1') return {operationId:'syndocal.query.output.video.authoring.lease.v1',fence:{...expiryFence},status:expiredReleased?'unavailable':'expired',authority:expiredReleased?null:{lease_id:'lease-0000000000000001',generation:4},resources:expiredReleased?[]:['video'],projectMatches:expiredReleased?null:true,managed:false};
 if(command==='query_output_control_authority_v1') return {operation_id:'syndocal.query.output.control.authority.v1',fence:{...expiryFence}};
 if(command==='allocate_output_control_request_id_v1') return 101;
 if(command==='relinquish_output_lease_v2') {expiredReleased=true;return {type:'receipt',receipt:{operation_id:args.request.operation_id,request_id:101,shape_sha256:'a'.repeat(64),argument_fingerprint:'b'.repeat(64),audit_sequence:1,fence_before:{...expiryFence},fence_after:{...expiryFence},outcome:'no_op',lease_result:{authority:{lease_id:'lease-0000000000000001',generation:5},resources:['video'],phase:'unclaimed',outcome:'relinquished',audit_sequence:1,changes:[{lease_id:'lease-0000000000000001',before_generation:4,after_generation:5,before_phase:'held_active',after_phase:'unclaimed',before_resources:['video'],after_resources:[]}]}}};}
 if(command==='get_show_clock_status') return {running:active,role:active?'standby':null,state:fail?'STALE':active?'LOCKED':'STOPPED',local_address:null,peer_address:active?'192.168.1.2:44666':null,session_id:null,node_id:null,clock_generation:1,fencing_generation:1,accepted_samples:active?55:0,last_sequence:0,offset_us:0,sample_age_us:active?1500:null,output_armed:false,show_clock_gate_armed:false,fence_state:'DISARMED',accepted_actions:0,scheduled_actions:0,last_action_sequence:0,last_action_id:null,last_action_status:null,dropped_datagrams:0,rejected_packets:fail?4:0,last_rejection:null,last_error:fail?'Clock peer is unavailable':null};
 if(command==='ltc_input_status_v1') return {running:active,signal:!fail,timecode:active?'01:02:03:04':null,frame_rate:30,device_name:'Fixture audio',channel_index:0,last_error:fail?'No timecode on the selected channel':null};
 if(command==='list_ltc_input_devices_v1') return [{name:'Fixture audio',channels:2,is_default:true}];
 if(command==='video_output_recording_status') return {takeover:{state:fail?'blocked':'idle',folder:null,reason:fail?'Recording folder is unavailable':null}};
 if(command==='set_output_ownership_role'&&args.role==='Standby') {ownership={...ownership,role:'Standby',effective_role:'Standby',desired_role:'Standby',persisted_role:'Standby',generation:6,epoch:5,lighting_allowed:false,video_allowed:false,lighting_reason:'Standby',video_reason:'Standby',error:null};return {...ownership};}
 throw Error('Fixture does not allow mutation: '+command);
};
window.__syncFixture={setMode:(next)=>{mode=next;},setLocale:(next)=>{setLocale(next);localizer.refresh();},log};
render(()=> <div class='syncFixtureShell layoutSetup'><header class='syncFixtureHeader'>SETUP / I/O / SYNC</header><section class='setupIoPanel' data-io-layout='operator'><div class='setupIoWorkbenchBody'><section data-io-zone='sync' class='ioUnifiedZone'><div class='ioOperatorSurface syncOperatorSurface'><StandbySyncPanel backendAvailable invokeCommand={invoke}/><ShowClockStatusPanel backendAvailable invokeCommand={invoke}/><LtcInputPanel backendAvailable invokeCommand={invoke}/></div></section></div></section></div>,document.getElementById('root'));
const localizer=installUiLocalization(document.body,locale);
window.__mountOutputActions=()=>{
 document.querySelector('.syncFixtureShell').style.display='none';
 const host=document.createElement('section');document.body.append(host);
 const [output,setOutput]=createSignal({id:42,kind:'Display'});
 const [actualOpen,setActualOpen]=createSignal(false);
 const [busy,setBusy]=createSignal(false);const [blocked,setBlocked]=createSignal(false);const actions=[];
 render(()=> <VideoOutputActionsPanel output={output()} actualOpen={actualOpen()} busy={busy()} blocked={blocked()} error={null} onToggleWindow={(id,open)=>actions.push({id,open})}/>,host);
 window.__outputFixture={setOutput,setActualOpen,setBusy,setBlocked,actions};
};
window.__syncReady=true;
`;
const plugin={name:'sync-disclosure-fixture',resolveId(id){if(id===fixtureId)return id;},load(id){if(id===fixtureId)return fixture;},configureServer(server){server.middlewares.use((req,res,next)=>{if(req.url!=='/__sync')return next();res.setHeader('Content-Type','text/html');res.end(`<!doctype html><html data-theme='tally'><head><title>Sync disclosure fixture</title><link rel="icon" href="data:,"><style>html,body{margin:0;overflow:auto}.syncFixtureShell{padding:16px;min-height:100vh;box-sizing:border-box}.syncFixtureHeader{min-height:150px;padding:8px}.setupIoWorkbenchBody{overflow:visible!important;max-height:none!important}.syncFixtureShell .setupIoPanel{display:block;width:100%;padding:8px}.syncOperatorSurface{display:grid;gap:8px}.syncFixtureShell button,.syncFixtureShell select,.syncFixtureShell input:not([type=checkbox]){min-height:44px}</style></head><body><div id='root'></div><script type='module' src='${fixtureId}'></script></body></html>`);});}};
const server=await createServer({root,plugins:[plugin],logLevel:'error',server:{host:'127.0.0.1',port:0}});
await server.listen();
const executablePath=['C:/Program Files/Google/Chrome/Application/chrome.exe','C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe'].find(existsSync);
const browser=await chromium.launch({headless:true,...(executablePath?{executablePath}:{})});
const errors=[]; const captures=[]; const reports=[];
try {
 const page=await browser.newPage();
 page.on('pageerror',error=>errors.push(String(error)));
 page.on('console',msg=>{if(msg.type()==='error')errors.push(msg.text());});
 for(const [width,height] of [[1920,1080],[1366,768],[1280,800]]) for(const locale of ['en','ja']) {
  await page.setViewportSize({width,height});
  await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/__sync`);
  await page.waitForFunction(()=>window.__syncReady===true);
  await page.evaluate(locale=>window.__syncFixture.setLocale(locale),locale);
  await page.waitForTimeout(150);
  const state=await page.evaluate(()=>({title:document.title,overlays:document.querySelectorAll('vite-error-overlay').length,disclosures:[...document.querySelectorAll('[data-sync-disclosure]')].map(el=>({id:el.dataset.syncDisclosure,open:el.open,summaryHeight:el.querySelector('summary').getBoundingClientRect().height})),bottom:document.querySelector('.syncOperatorSurface').getBoundingClientRect().bottom,scrollWidth:document.documentElement.scrollWidth,buttonHeight:document.querySelector('[data-io-control=enable-output]').getBoundingClientRect().height,font:getComputedStyle(document.querySelector('[data-sync-disclosure=show-clock] h2')).fontSize,baselineFont:(()=>{const host=document.createElement('header');host.className='ioDeskHeader';host.innerHTML='<h2 class="textBalance">ShowClock LAN</h2>';document.querySelector('.showClockPanel').append(host);const size=getComputedStyle(host.firstElementChild).fontSize;host.remove();return size;})(),text:document.querySelector('.syncOperatorSurface').innerText}));
  assert.equal(state.title,'Sync disclosure fixture');assert.equal(state.overlays,0);
  assert.equal(state.disclosures.length,4);assert.ok(state.disclosures.every(row=>!row.open&&row.summaryHeight>=44));
  assert.ok(state.bottom<=height,`${width}/${locale} default Sync fits without scrolling: ${state.bottom}`);
  assert.ok(state.scrollWidth<=width,`${width}/${locale} horizontal containment`);
  assert.ok(state.buttonHeight>=44); assert.ok(parseFloat(state.font)>=parseFloat(state.baselineFont),`feature heading preserves title size: ${state.font}/${state.baselineFont}`);
  assert.ok(!/Samples|Shared Folder|Pairing key|µs/.test(state.text),'inactive metrics and technical setup are absent from default view');
  const path=resolve(artifacts,`sync-default-${locale}-${width}x${height}.png`);await page.screenshot({path,fullPage:true});captures.push(path);reports.push({width,height,locale,...state});
 }
 // Keyboard opening reveals configuration but must never send a control action.
 const before=await page.evaluate(()=>window.__syncFixture.log.filter(x=>!['standby_sync_status','get_output_ownership_status','query_output_lease_authority_v1','get_show_clock_status','ltc_input_status_v1','list_ltc_input_devices_v1','video_output_recording_status'].includes(x.command)).length);
 const clock=page.locator('[data-sync-disclosure=show-clock]');await clock.locator('summary').first().focus();await page.keyboard.press('Enter');assert.equal(await clock.getAttribute('open'),'');
 await clock.locator('details').first().locator('summary').click();assert.equal(await clock.locator('input[type=password]').isVisible(),true);
 await clock.locator('summary').first().click();
 await page.locator('[data-sync-disclosure=ltc-input]>summary').click();assert.equal(await page.locator('[data-io-control=ltc-input-start]').isVisible(),true);await page.locator('[data-sync-disclosure=ltc-input]>summary').click();
 const after=await page.evaluate(()=>window.__syncFixture.log.filter(x=>!['standby_sync_status','get_output_ownership_status','query_output_lease_authority_v1','get_show_clock_status','ltc_input_status_v1','list_ltc_input_devices_v1','video_output_recording_status'].includes(x.command)).length);assert.equal(after,before,'opening setup never starts services or outputs');
 await page.evaluate(()=>window.__syncFixture.setMode('active'));await page.waitForTimeout(2200);
 assert.equal(await page.locator('[data-show-clock-shell-state=locked]').count(),1);assert.equal(await page.locator('.ltcInputState').isVisible(),true);
 assert.equal(await page.locator('[data-sync-disclosure][open]').count(),0,'polling does not force configuration open');
 await page.evaluate(()=>window.__syncFixture.setMode('failure'));await page.waitForTimeout(2200);
 for(const text of ['Shared folder is unavailable','Clock peer is unavailable','No timecode on the selected channel','Recording folder is unavailable']) assert.equal(await page.getByText(text,{exact:false}).first().isVisible(),true,`${text}: failure remains visible while setup is closed`);
 await page.screenshot({path:resolve(artifacts,'sync-failures-ja-1280x800.png'),fullPage:true});
 await page.locator('[data-io-control=standby-output]').click();await page.waitForTimeout(80);assert.equal(await page.evaluate(()=>window.__syncFixture.log.filter(x=>x.command==='set_output_ownership_role'&&x.args.role==='Standby').length),1,'normal Standby uses the existing exact role transition');
 await page.evaluate(()=>{window.__syncFixture.setMode('expired');window.__syncFixture.setLocale('en');});
 const expiryAction=page.locator('[data-io-expired-video-lease] button');
 await expiryAction.waitFor({state:'visible'});
 await expiryAction.click();await expiryAction.waitFor({state:'hidden'});
 assert.equal(await page.evaluate(()=>window.__syncFixture.log.filter(x=>x.command==='relinquish_output_lease_v2').length),1,'expired claim can be explicitly retired');
 assert.equal(await page.evaluate(()=>window.__syncFixture.log.filter(x=>x.command==='enable_output_control_v2').length),0,'cleanup does not enable lighting or video');
 assert.ok(await page.locator('.syncOutputPanel').innerText().then(text=>text.includes('Standby')),'cleanup remains Standby');
 // Real Display action component must track later props, not only its mount snapshot.
 await page.evaluate(()=>{window.__syncFixture.setLocale('en');window.__mountOutputActions();});
 const action=page.locator('[data-action=set-display-window-open]');
 await action.waitFor();assert.equal(await action.innerText(),'Open/Reopen output');
 await page.evaluate(()=>window.__outputFixture.setActualOpen(true));assert.equal(await action.innerText(),'Close output window');
 await page.evaluate(()=>window.__outputFixture.setBusy(true));assert.equal(await action.isDisabled(),true);assert.equal(await action.innerText(),'Closing output window…');
 await page.evaluate(()=>{window.__outputFixture.setBusy(false);window.__outputFixture.setActualOpen(null);});assert.equal(await action.isDisabled(),true);
 await page.evaluate(()=>{window.__outputFixture.setActualOpen(false);window.__outputFixture.setBlocked(true);});assert.equal(await action.isDisabled(),true);
 await page.evaluate(()=>window.__outputFixture.setOutput({id:42,kind:'SpoutSender'}));assert.equal(await action.count(),0,'later non-Display props hide the Display action');
 await page.evaluate(()=>{window.__outputFixture.setOutput({id:42,kind:'Display'});window.__outputFixture.setBlocked(false);});await action.click();assert.deepEqual(await page.evaluate(()=>window.__outputFixture.actions),[{id:42,open:true}]);
 assert.deepEqual(errors,[],'no browser runtime errors');
 await writeFile(resolve(artifacts,'results.json'),JSON.stringify({passed:true,browserPlugin:'not available; existing Playwright fallback',reports,captures,errors},null,2));
 console.log(`Sync disclosure browser: PASS (6 viewport/locales, default fits, keyboard setup, no accidental sends, active/failure visibility, Display action prop updates); evidence ${artifacts}`);
} finally {await browser.close();await server.close();}
