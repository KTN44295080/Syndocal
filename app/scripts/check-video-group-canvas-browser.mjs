// Browser plugin not available. Use the repository's owned Chromium/CDP harness.
// Flow: Edit inspector -> select group -> drag canvas -> one ticket/Undo record
// and updated inspector values. IPC/history is a strict in-memory backend fixture.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";
import { reserveBrowserCheckPort } from "./browser-check-ports.mjs";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const evidence = resolve(appRoot, "../target/qa/task31-group-canvas");
const fixturePath = "/src/__group-canvas-fixture.tsx";
const fixtureId = join(appRoot, fixturePath.slice(1)).replaceAll("\\", "/");
const fixture = `
import { createSignal } from 'solid-js';
import { render } from 'solid-js/web';
import { EditVideoInspector } from '/src/components/EditVideoInspector.tsx';
import { createVideoRuntimeController } from '/src/createVideoRuntimeController.ts';
import { defaultTransform } from '/src/videoLayerDefaults.ts';
import { installUiLocalization } from '/src/uiLocalization.ts';
import '/src/theme.css'; import '/src/styles.css';
const initial={effect_chains:[],effect_presets:[],transition_buses:[],layer_groups:[{id:31,label:'Canvas test group',composition_id:2,layer_ids:[1],enabled:true,opacity:1,blend_mode:'Normal',solo:false,bypassed:false,transform:{...defaultTransform,scale_x:0.6,scale_y:0.4}}]};
const [catalog,setCatalog]=createSignal(structuredClone(initial));
const [authority,setAuthority]=createSignal({project_epoch:1,project_revision:10,checkpoint_hash:'h10'});
const calls=[],undo=[],previews=[],messages=[];let failure=false,delay=null;
const same=(a,b)=>JSON.stringify(a)===JSON.stringify(b);
const controller=createVideoRuntimeController({
 snapshot:()=>({video:{...catalog(),layers:[]}}),getCurrentProjectAuthority:authority,
 isProjectAuthorityCurrent:a=>same(a,authority()),authoritativeApplicationIsCurrent:r=>same(r.mutation.authority,authority()),
 prepareMediaAssetOperationStart:async expected=>{if(!same(expected,authority()))throw Error('stale preflight');return {authority:authority()};},
 refreshSnapshot:async()=>({}),isIsfEventPulseBusy:()=>false,setMessage:m=>messages.push(m),setVideoPreviewDiagnostics:()=>{},
 invoke:async(command,args)=>{
  if(command==='get_video_preview_diagnostics')return {};
  if(command==='get_debug_video_preview'){
    if(args.compositionId!==2)throw Error('wrong composition');previews.push(args);
    return {width:2,height:2,format:'Rgba8',data:[32,48,64,255,80,120,160,255,80,120,160,255,32,48,64,255]};
  }
  if(command==='get_video_effect_catalog_operation_terminal_result')return null;
  if(command!=='apply_video_effect_catalog_authoritative')throw Error('unexpected '+command);
  if(!Number.isSafeInteger(args.requestId)||args.expectedRevision!==authority().project_revision||args.expectedCheckpointHash!==authority().checkpoint_hash||args.__expectedProjectEpoch!==authority().project_epoch)throw Error('invalid ticket');
  calls.push(structuredClone(args)); if(delay)await new Promise(resolve=>delay.resolve=resolve);
  if(failure)throw Error('fixture rejection');
  undo.push(structuredClone(catalog()));setCatalog(structuredClone(args.request));
  setAuthority({...authority(),project_revision:authority().project_revision+1,checkpoint_hash:'h'+(authority().project_revision+1)});
  return {catalog:catalog(),mutation:{authority:authority()}};
 }
});
const effects={get catalog(){return catalog()},get canvasAuthority(){return JSON.stringify(authority())},compositions:[{id:2,label:'Second composition',layer_ids:[1],output_ids:[9]}],outputs:[{id:9,width:1920,height:1080}],transitionRuntime:{buses:[]},
onApplyCatalog:controller.applyVideoEffectCatalog,onLoadGroupPreview:controller.loadVideoGroupCanvasPreview,onImportIsf:()=>{},onApplyPreset:()=>{},onRemoveChain:()=>{}};
render(()=><div style={{'max-width':'800px',margin:'0 auto'}}><EditVideoInspector asset={null} availabilityById={{}} layerFx={{layers:[]}} groupEffects={effects}/></div>,document.getElementById('root'));
const localization=installUiLocalization(document.getElementById('root'),()=>document.documentElement.dataset.locale??'en');
Object.assign(window,{__canvas:{calls,undo,previews,messages,catalog,authority,
 undoLast:()=>{if(undo.length){setCatalog(undo.pop());setAuthority({...authority(),project_revision:authority().project_revision+1,checkpoint_hash:'undo'});}},
 interleave:()=>setAuthority({...authority(),project_revision:authority().project_revision+1,checkpoint_hash:'interleaved'}),
 reject:value=>failure=value,hold:()=>delay={},release:()=>{delay?.resolve?.();delay=null},
 locale:value=>{document.documentElement.dataset.locale=value;localization.refresh()},
 extreme:()=>{setCatalog({...catalog(),layer_groups:[{...catalog().layer_groups[0],transform:{...defaultTransform,scale_x:3,scale_y:3,x:1,y:-1,rotation_deg:45}}]});},
 reset:()=>{setCatalog(structuredClone(initial));setAuthority({project_epoch:1,project_revision:10,checkpoint_hash:'h10'});calls.length=0;undo.length=0;},
}});
`;
const plugin = { name: "group-canvas-check", enforce: "pre", resolveId(id) { if (id === fixturePath) return fixtureId; }, load(id) { if (id === fixtureId) return fixture; }, configureServer(server) {
  server.middlewares.use((req, res, next) => { if (req.url !== "/__group-canvas") return next(); res.setHeader("Content-Type", "text/html"); res.end('<!doctype html><meta name="viewport" content="width=device-width,initial-scale=1"><title>Group canvas check</title><div id="root"></div><script type="module" src="'+fixturePath+'"></script>'); });
}};
const sleep = ms => new Promise(done => setTimeout(done, ms));
async function waitFor(check, label, timeout=30000) { const end=Date.now()+timeout;let last;while(Date.now()<end){try{const value=await check();if(value)return value;}catch(e){last=e;}await sleep(80);}throw Error(`Timed out: ${label} ${last??''}`); }
class Cdp {
  next=0; pending=new Map(); errors=[];
  async connect(url) {
    this.socket=new WebSocket(url);await new Promise((done,reject)=>{this.socket.addEventListener('open',done,{once:true});this.socket.addEventListener('error',reject,{once:true});});
    this.socket.addEventListener('message',({data})=>{const m=JSON.parse(data);if(m.method==='Runtime.exceptionThrown')this.errors.push(m.params);if(m.method==='Runtime.consoleAPICalled'&&['error','warning'].includes(m.params.type))this.errors.push(m.params);const p=this.pending.get(m.id);if(p){clearTimeout(p.timer);this.pending.delete(m.id);m.error?p.reject(Error(JSON.stringify(m.error))):p.done(m.result);}});
  }
  send(method,params={}) {return new Promise((done,reject)=>{const id=++this.next;const timer=setTimeout(()=>{this.pending.delete(id);reject(Error('CDP timeout '+method));},20000);this.pending.set(id,{done,reject,timer});this.socket.send(JSON.stringify({id,method,params}));});}
  async evaluate(expression) {const r=await this.send('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});assert.ok(!r.exceptionDetails,JSON.stringify(r.exceptionDetails));return r.result.value;}
}
let server,browser,profile,cdp;
const results=[];
try {
  await mkdir(evidence,{recursive:true});
  const vitePort=await reserveBrowserCheckPort('SYNDOCAL_VIEWPORT_VITE_PORT');
  const cdpPort=await reserveBrowserCheckPort('SYNDOCAL_CDP_PORT');
  server=await createServer({root:appRoot,cacheDir:join(evidence,'vite-cache'),server:{host:'127.0.0.1',port:vitePort,strictPort:true},plugins:[plugin],clearScreen:false});await server.listen();
  const executable=[process.env.CHROME_PATH,'C:/Program Files/Google/Chrome/Application/chrome.exe','C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe'].filter(Boolean).find(existsSync);assert.ok(executable);
  profile=await mkdtemp(join(tmpdir(),'syndocal-group-canvas-'));
  browser=spawn(executable,['--headless=new','--disable-gpu','--no-first-run','--no-default-browser-check',`--remote-debugging-port=${cdpPort}`,`--user-data-dir=${profile}`,'about:blank'],{windowsHide:true,stdio:'ignore'});
  browser.on('error',error=>{throw error});
  const target=await waitFor(async()=>{const r=await fetch(`http://127.0.0.1:${cdpPort}/json/list`,{signal:AbortSignal.timeout(1000)});return(await r.json()).find(t=>t.type==='page');},'browser');
  cdp=new Cdp();await cdp.connect(target.webSocketDebuggerUrl);await cdp.send('Runtime.enable');await cdp.send('Page.enable');
  await cdp.send('Emulation.setDeviceMetricsOverride',{width:1400,height:1000,deviceScaleFactor:1,mobile:false});
  const url=`http://127.0.0.1:${vitePort}/__group-canvas`;await cdp.send('Page.navigate',{url});
  await waitFor(()=>cdp.evaluate('!!window.__canvas && !!document.querySelector("[data-video-group-edit]")'),'fixture');
  assert.deepEqual(await cdp.evaluate('({url:location.href,title:document.title,overlay:!!document.querySelector("vite-error-overlay")})'),{url,title:'Group canvas check',overlay:false});
  await cdp.evaluate('document.querySelector("[data-video-group-edit]").click();window.__events=[];for(const type of ["pointerdown","pointermove","pointerup","pointercancel","lostpointercapture"])document.addEventListener(type,e=>__events.push({type,id:e.pointerId,target:e.target.className.baseVal??e.target.className,x:e.clientX,y:e.clientY}),true)');
  await waitFor(()=>cdp.evaluate('!!document.querySelector("[data-video-group-canvas] image")'),'selected group preview');
  const stats=()=>cdp.evaluate('({calls:__canvas.calls.length,undo:__canvas.undo.length,t:__canvas.catalog().layer_groups[0].transform,inputs:Array.from(document.querySelectorAll(".videoGroupTransformFields input")).map(e=>Number(e.value))})');
  const prepare=async(selector='.videoGroupCanvasBox')=>{await waitFor(()=>cdp.evaluate('!document.querySelector("[data-group-handle=rotate]").disabled'), 'canvas ready');return cdp.evaluate(`(()=>{const e=document.querySelector(${JSON.stringify(selector)});e.scrollIntoView({block:'center'});const r=e.getBoundingClientRect(),s=document.querySelector('.videoGroupCanvasViewport').getBoundingClientRect();return {x:r.x+r.width/2,y:r.y+r.height/2,w:s.width,h:s.height,units:Number(document.querySelector(".videoGroupCanvasViewport svg").viewBox.baseVal.width)};})()`);};
  const mouse=(type,x,y,shift=false)=>cdp.send('Input.dispatchMouseEvent',{type,x,y,button:'left',buttons:type==='mouseReleased'?0:1,clickCount:1,modifiers:shift?8:0});
  const key=async(key,shift=false)=>{await cdp.send('Input.dispatchKeyEvent',{type:'keyDown',key,code:key,modifiers:shift?8:0});await cdp.send('Input.dispatchKeyEvent',{type:'keyUp',key,code:key,modifiers:shift?8:0});};
  let p=await prepare();await mouse('mousePressed',p.x,p.y);for(let i=1;i<=8;i++)await mouse('mouseMoved',p.x+i*4,p.y+i*2);
  let during=await stats();assert.equal(during.calls,0,'drag coalesces locally');assert.ok(during.inputs[0]>0,'inspector follows live draft');
  await mouse('mouseReleased',p.x+32,p.y+16);await waitFor(async()=>!(await cdp.evaluate('document.querySelector(".videoGroupTransformFields input").disabled'))&&(await stats()).calls===1,'commit');
  let after=await stats();assert.equal(after.undo,1,'one drag = one Undo');assert.ok(Math.abs(after.t.x-32*p.units/p.w)<1e-6);assert.ok(Math.abs(after.inputs[0]-after.t.x)<0.00005);assert.ok(Math.abs(after.inputs[1]-after.t.y)<0.00005);
  await cdp.evaluate('__canvas.undoLast()');assert.equal((await stats()).t.x,0);results.push('move: live inspector, one ticket/Undo, undo restores start');
  p=await prepare();await mouse('mousePressed',p.x,p.y);await mouse('mouseMoved',p.x+40,p.y+20);await key('Escape');await mouse('mouseReleased',p.x+40,p.y+20);after=await stats();assert.equal(after.calls,1);assert.equal(after.t.x,0);assert.equal(after.inputs[0],0);results.push('Escape restores start with no publication');
  p=await prepare('[data-group-handle="se"]');await mouse('mousePressed',p.x,p.y);await mouse('mouseMoved',p.x+25,p.y+10,true);await mouse('mouseReleased',p.x+25,p.y+10,true);await waitFor(async()=>(await stats()).calls===2,'scale');after=await stats();assert.ok(after.t.scale_x>0.6);assert.ok(Math.abs(after.t.scale_x/after.t.scale_y-1.5)<1e-8);assert.equal(after.undo,1);results.push('corner scaling keeps aspect with Shift');
  p=await prepare('[data-group-handle="rotate"]');await mouse('mousePressed',p.x,p.y);await mouse('mouseMoved',p.x+35,p.y+8,true);await mouse('mouseReleased',p.x+35,p.y+8,true);await waitFor(async()=>(await stats()).calls===3,'rotate');after=await stats();assert.notEqual(after.t.rotation_deg,0);assert.equal(after.t.rotation_deg%15,0);results.push('rotation snaps to 15 degrees');
  await cdp.evaluate('document.querySelector(".videoGroupCanvasBox").focus()');const x=after.t.x;await key('ArrowRight',true);await waitFor(async()=>(await stats()).calls===4,'nudge');assert.ok(Math.abs((await stats()).t.x-x-0.1)<1e-8);results.push('keyboard Shift nudge');
  p=await prepare();await mouse('mousePressed',p.x,p.y);await mouse('mouseMoved',p.x+20,p.y);await cdp.evaluate('__canvas.interleave()');await mouse('mouseReleased',p.x+20,p.y);assert.equal((await stats()).calls,4);assert.equal((await stats()).inputs[0],(await stats()).t.x);results.push('authority interleaving cancels draft');
  p=await prepare();await mouse('mousePressed',p.x,p.y);await mouse('mouseMoved',p.x+20,p.y);await cdp.evaluate('document.querySelector(".videoGroupCanvasBox").dispatchEvent(new PointerEvent("pointercancel",{bubbles:true,pointerId:1}))');await mouse('mouseReleased',p.x+20,p.y);assert.equal((await stats()).calls,4);results.push('pointer cancellation releases draft');
  await cdp.evaluate('__canvas.reject(true)');p=await prepare();await mouse('mousePressed',p.x,p.y);await mouse('mouseReleased',p.x+20,p.y);await waitFor(()=>cdp.evaluate('document.querySelector(".videoGroupCanvas [role=alert]")?.textContent.includes("not applied")'),'failed publication message');after=await stats();assert.equal(after.calls,5);assert.equal(after.undo,3);assert.equal(after.inputs[0],after.t.x);await cdp.evaluate('__canvas.reject(false)');results.push('rejected publication discards optimistic values');
  p=await prepare();await mouse('mousePressed',p.x,p.y);await mouse('mouseReleased',p.x,p.y);assert.equal((await stats()).calls,5);results.push('no-op click has no publication');
  await cdp.evaluate('__canvas.hold()');p=await prepare();await mouse('mousePressed',p.x,p.y);await mouse('mouseReleased',p.x+20,p.y);
  await waitFor(()=>cdp.evaluate('__canvas.calls.length===6 && document.querySelector("[data-group-handle=rotate]").disabled'),'pending publication');
  await mouse('mousePressed',p.x,p.y);await mouse('mouseReleased',p.x+30,p.y);await key('ArrowRight');assert.equal((await stats()).calls,6,'pending edit cannot duplicate publication');
  await cdp.evaluate('__canvas.release()');await waitFor(()=>cdp.evaluate('!document.querySelector("[data-group-handle=rotate]").disabled'),'publication settled');assert.equal((await stats()).undo,4);results.push('pending publication blocks duplicate gestures');
  for(const [width,height,locale] of [[1400,1000,'en'],[390,844,'ja']]) {
    await cdp.send('Emulation.setDeviceMetricsOverride',{width,height,deviceScaleFactor:1,mobile:false});
    await cdp.evaluate(`__canvas.locale('${locale}');document.querySelector('.videoGroupCanvas').scrollIntoView({block:'center'});`);
    assert.ok(await cdp.evaluate('document.documentElement.scrollWidth<=innerWidth'),'no horizontal clipping');
    assert.ok(await cdp.evaluate('Array.from(document.querySelectorAll(".videoGroupCanvasHandle")).every(e=>e.getBoundingClientRect().width>=44&&e.getAttribute("aria-label"))'));
    assert.ok(await cdp.evaluate('(()=>{const e=document.querySelector(".videoGroupCanvasViewport").getBoundingClientRect();return Math.abs(e.width/e.height-16/9)<0.01})()'),'canvas keeps composition aspect');
    assert.ok(await cdp.evaluate('parseFloat(getComputedStyle(document.querySelector(".videoGroupCanvas p")).fontSize)>=13'));
    assert.ok(await cdp.evaluate('!document.querySelector("vite-error-overlay")'));
    if(locale==='ja')assert.ok(await cdp.evaluate('document.querySelector(".videoGroupCanvas header strong").textContent!=="Group canvas"'));
    const image=await cdp.send('Page.captureScreenshot',{format:'png'});await writeFile(join(evidence,`canvas-${width}-${locale}.png`),Buffer.from(image.data,'base64'));
  }
  await cdp.evaluate('__canvas.extreme()');
  await waitFor(()=>cdp.evaluate('(()=>{const v=document.querySelector(".videoGroupCanvasViewport").getBoundingClientRect();return Array.from(document.querySelectorAll(".videoGroupCanvasHandle")).every(e=>{const r=e.getBoundingClientRect();return r.left>=v.left&&r.right<=v.right&&r.top>=v.top&&r.bottom<=v.bottom})})()'),'large transform handle containment');
  results.push('scale=3, rotated and translated: all 44px targets fully contained at 390px');
  assert.equal(cdp.errors.length,0,JSON.stringify(cdp.errors));
  await writeFile(join(evidence,'results.json'),JSON.stringify({results,consoleErrors:cdp.errors,stats:await stats(),previews:await cdp.evaluate('__canvas.previews'),limitations:'Strict backend fixture; no native window/hardware acceptance.'},null,2));
  console.log('PASS group canvas browser: '+results.join('; ')+'; desktop/mobile, JA/EN, 44px targets, no console errors');
} catch(error) {
  if(cdp){await writeFile(join(evidence,'failure.json'),JSON.stringify({error:error.stack,errors:cdp.errors,state:await cdp.evaluate('({events:window.__events,stats:window.__canvas?{calls:__canvas.calls.length,undo:__canvas.undo.length,t:__canvas.catalog().layer_groups[0].transform}:null})').catch(()=>null),body:await cdp.evaluate('document.body.innerText').catch(()=>null)},null,2));const image=await cdp.send('Page.captureScreenshot',{format:'png'}).catch(()=>null);if(image)await writeFile(join(evidence,'failure.png'),Buffer.from(image.data,'base64'));}
  throw error;
} finally {
  if(cdp?.socket.readyState===WebSocket.OPEN){await cdp.send('Browser.close').catch(()=>{});cdp.socket.close();}
  if(browser&&browser.exitCode===null){await waitFor(()=>browser.exitCode!==null||browser.signalCode!==null,'owned browser exit',5000).catch(()=>browser.kill());}
  await server?.close();
  if(profile){const target=resolve(profile);assert.equal(dirname(target),resolve(tmpdir()));assert.ok(basename(target).startsWith('syndocal-group-canvas-'));await rm(target,{recursive:true,force:true,maxRetries:5,retryDelay:200});}
}
