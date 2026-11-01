// Native file-drop routing + real Perform/Media Library hover fixtures.
// Runs in Chromium with a mock WebView2 path resolver; never launches Syndocal.
import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { existsSync } from "node:fs";
import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";
const require = createRequire(import.meta.url);
let chromium;
try { ({ chromium } = require("playwright")); } catch {
  ({ chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH ?? path.join(homedir(), ".cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright")));
}
const appRoot = fileURLToPath(new URL("../", import.meta.url));
// Enabling wry's OLE interceptor regresses native HTML5 dragging (dd9d9f44).
// File paths now come from WebView2 AdditionalObjects, so retain this setting
// in production and the MCP fixture windows that use this configuration.
for (const name of (await readdir(path.join(appRoot, "src-tauri"))).filter(name => /^tauri(?:\.mcp-[\w-]+)?\.conf\.json$/.test(name))) {
  const config = JSON.parse(await readFile(path.join(appRoot, "src-tauri", name), "utf8"));
  for (const window of config.app?.windows ?? []) {
    assert.equal(window.dragDropEnabled, false, `${name}: preserve HTML5 in-app dragging`);
  }
}
const evidence = path.resolve(process.env.NATIVE_DROP_QA_DIR ?? path.join(appRoot, "../target/qa/ui-usability-20261028/native-drop"));
await mkdir(evidence, { recursive: true });
const chrome = [process.env.CHROME_PATH, "C:/Program Files/Google/Chrome/Application/chrome.exe", "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe", chromium.executablePath()].find(p => p && existsSync(p));
assert.ok(chrome, "Chromium is required");
const modulePath = "/src/__native-file-drop-fixture.tsx";
const moduleId = path.join(appRoot, modulePath.slice(1)).replaceAll("\\", "/");
const fixture = String.raw`
import { createSignal } from "solid-js";
import { render } from "solid-js/web";
import { VideoPerformMatrix } from "/src/components/VideoPerformMatrix";
import { VideoControlPanel } from "/src/components/VideoControlPanel";
import { routeFileDrop } from "/src/fileDropRouting";
import { installNativeFileDrop, unsupportedFileDropMessage } from "/src/nativeFileDrop";
import { installUiLocalization } from "/src/uiLocalization";
import "/src/theme.css"; import "/src/styles.css";
const locale = new URL(location.href).searchParams.get("locale") || "en";
const calls = [], requests = [], listeners = new Set();
const [status, setStatus] = createSignal("");
let context = 1;
const actions = Object.fromEntries(["firstShow", "importMedia", "assignSlots", "openProject", "addAudio"].map(name => [name, (...args) => calls.push({name,args})]));
actions.message = message => { calls.push({name:"message",args:[message]}); setStatus(message); };
const webview = {
  postMessageWithAdditionalObjects(message, files) { requests.push({message, files:files.map(file=>file.name)}); },
  addEventListener(type, listener) { listeners.add(listener); },
  removeEventListener(type, listener) { listeners.delete(listener); },
};
const bank = {mode:"control",layers:[],layer:null,runtime:null,assets:[],thumbnails:{},selectedSlotId:null,onSelectLayer(){},onSelect(){},onQueue(){},onAssign(){},onReorder(){},onPatch(){},onDuplicate(){},onClear(){},onImportMedia(){}};
const library = {assets:[],thumbnails:{},availabilityById:{},activeOperations:[],lastImportReport:null,backendAvailable:true,onVerify(){},onRelink(){},onCancelOperation(){}};
render(() => <main style={{padding:"16px",display:"grid",gap:"24px"}}>
  <div style={{height:"240px", "min-width":"0"}}><VideoPerformMatrix bank={bank} layerList={{layers:[]}} runtimes={[]} availabilityById={{}} previewLayerId={null} thumbnailsAuthorized={false} onStop={()=>{}} onTake={()=>{}}
    firstShow={{available:true,backendAvailable:true,busy:false,onStart:(paths)=>calls.push({name:"pickerFallback",args:[paths]})}} /></div>
  <VideoControlPanel mixer={false} libraryOnly={true} layerCount={0} mediaLibrary={library} clipSlotBank={bank} clipGrid={{layers:[],thumbnailsAuthorized:false,recordingStatus:{state:"Idle",active:false,admission_allowed:false},selectedOutputId:null}} sourceCreate={{sourceKind:"File",label:"",path:""}} />
  <output data-status>{status()}</output>
  <div data-internal onDrop={event=>{ if(event.dataTransfer?.getData("application/syndocal-test")) {event.preventDefault();calls.push({name:"internal",args:[event.dataTransfer.getData("application/syndocal-test")]});}}}>Internal drag fixture</div>
  <div hidden><div data-project-drop-surface/><div data-timeline-audio-layer-id="8"/><div data-video-clip-slot-layer-id="3" data-video-clip-slot-id="4"/></div>
</main>, document.getElementById("root"));
installUiLocalization(document.body, () => locale);
const stop = installNativeFileDrop({document,webview,onDrop:(paths,target)=>routeFileDrop(paths,target,actions),onError:actions.message,captureContext:()=>context,contextIsCurrent:old=>old===context});
window.__dropFixture = {calls,requests,stop,unsupportedFileDropMessage,advanceContext:()=>context++,route:(paths,selector)=>routeFileDrop(paths,document.querySelector(selector),actions),
 reply:(paths, extra={})=>{const id=requests.at(-1).message.id; for(const listener of listeners) listener({data:{type:"syndocal:file-drop-result",id,paths,...extra}});}};
`;
const server = await createServer({root:appRoot,cacheDir:path.join(evidence,"vite-cache"),clearScreen:false,logLevel:"error",server:{host:"127.0.0.1",port:0},plugins:[{
 name:"native-drop-fixture",enforce:"pre",resolveId(id){if(id===modulePath)return moduleId;},load(id){if(id===moduleId)return fixture;},
 configureServer(server){server.middlewares.use((req,res,next)=>{if(!req.url?.startsWith("/__native-drop"))return next();res.setHeader("Content-Type","text/html");res.end(`<!doctype html><html data-theme="tally"><head><meta charset="utf-8"><title>Native file drop fixture</title><link rel="icon" href="data:,"><style>html,body{margin:0}main{box-sizing:border-box;max-width:100vw}</style></head><body><div id="root"></div><script type="module" src="${modulePath}"></script></body></html>`);});}
}]});
let browser;
const results=[];
const perform="[data-video-perform-first-show-drop]", library="[data-media-library-rail]";
const paths=["C:\\??\\intro.MP4","C:\\??\\still.png"];
try {
 await server.listen();
 browser=await chromium.launch({headless:true,executablePath:chrome});
 for(const locale of ["ja","en"]) for(const [width,height] of [[1920,1080],[960,640]]) {
  const page=await browser.newPage({viewport:{width,height}}); const errors=[];
  page.on("pageerror",error=>errors.push(error.stack));
  page.on("console",message=>{if(["error","warning"].includes(message.type()))errors.push(message.text());});
  const url=`${server.resolvedUrls.local[0]}__native-drop?locale=${locale}`;
  await page.goto(url); await page.waitForFunction(()=>Boolean(window.__dropFixture)).catch(error=>{throw new Error(`${error.message}; ${JSON.stringify(errors)}`);});
  assert.equal(await page.title(),"Native file drop fixture"); assert.equal(page.url(),url);
  assert.ok((await page.locator(perform).innerText()).length>0,"not blank");
  assert.equal(await page.locator("vite-error-overlay").count(),0);
  const fx=(fn,arg)=>page.evaluate(([code,arg])=>new Function("f","arg",code)(window.__dropFixture,arg),[fn,arg]);
  const drag=async(selector,phase="drop",files=["intro.mp4"])=>page.evaluate(({selector,phase,files})=>{
   const t=document.querySelector(selector),d=new DataTransfer(); files.forEach(name=>d.items.add(new File(["test"],name)));
   const event=new DragEvent(phase,{bubbles:true,cancelable:true,dataTransfer:d});t.dispatchEvent(event);
   return {prevented:event.defaultPrevented,highlighted:t.hasAttribute("data-file-drag-over"),outline:getComputedStyle(t.querySelector(".videoMediaLibrarySurface")??t).outlineStyle};
  },{selector,phase,files});
  for(const [name,selector] of [["perform",perform],["library",library]]) {
   const over=await drag(selector,"dragover"); assert.equal(over.prevented,true); assert.equal(over.highlighted,true);assert.equal(over.outline,"dashed");
   assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),true,"document fits viewport");
   const bounds=await page.locator(selector).boundingBox();assert.ok(bounds.x>=0&&bounds.x+bounds.width<=width&&bounds.y+bounds.height<=height,`${name} visible and contained`);
   await page.screenshot({path:path.join(evidence,`${locale}-${width}x${height}-${name}-hover.png`)});
   if(name==="library") assert.equal(await page.locator(`${selector} > .videoMediaLibrarySurface`).evaluate(e=>getComputedStyle(e).outlineStyle),"dashed","whole library surface highlights");
   const count=await fx("return f.calls.length");
   assert.equal((await drag(selector)).prevented,true);assert.equal(await page.locator(selector).getAttribute("data-file-drag-over"),null);
   assert.equal(await fx("return f.calls.length"),count,"wait for native paths; no picker fallback");
   await fx("f.reply(arg)",paths);
   assert.deepEqual(await fx("return f.calls.at(-1)"),{name:name==="perform"?"firstShow":"importMedia",args:[paths]});
   assert.equal(await fx("return f.calls.length"),count+1,"one gesture makes exactly one validated-path call");
   assert.equal((await drag(selector,"dragover")).highlighted,true,"hover works again after drop"); await drag(selector,"dragleave");
   for(const rejected of [["C:\\bad.exe"],[paths[0],"C:\\bad.exe"],["C:\\audio.wav"]]) {
    await drag(selector);await fx("f.reply(arg)",rejected);
    assert.deepEqual(await fx("return f.calls.at(-1)"),{name:"message",args:[await fx("return f.unsupportedFileDropMessage")]},"unsupported/mixed rejected before import");
   }
  }
  assert.equal((await drag("[data-internal]","dragover")).prevented,true,"external Files reach off-target rejection instead of WebView navigation");
  const count=await fx("return f.calls.length");
  await drag(library);await fx("f.advanceContext();f.reply(arg)",paths);assert.equal(await fx("return f.calls.length"),count,"connected old-project target cannot import into new project");
  await drag(library);const oldId=await fx("return f.requests.at(-1).message.id");await drag(library);
  await fx("f.reply(arg.paths,{id:arg.id})",{paths,id:oldId});assert.equal(await fx("return f.calls.length"),count,"older gesture ignored");
  await fx("f.reply(arg)",paths);assert.equal(await fx("return f.calls.length"),count+1);
  await drag(library);await fx("f.reply([], {error:true})");assert.equal(await fx("return f.calls.at(-1).name"),"message");
  await drag(library);await fx("f.reply([123])");assert.equal(await fx("return f.calls.at(-1).name"),"message");
  const beforeInternal=await fx("return f.requests.length");
  const internal=await page.evaluate(()=>{const d=new DataTransfer();d.setData("application/syndocal-test","profile/scene/FX/Timeline payload");const e=new DragEvent("drop",{bubbles:true,cancelable:true,dataTransfer:d});document.querySelector("[data-internal]").dispatchEvent(e);return e.defaultPrevented;});
  assert.equal(internal,true);assert.equal(await fx("return f.requests.length"),beforeInternal,"internal drag never enters native bridge");assert.equal(await fx("return f.calls.at(-1).name"),"internal");
  await fx("f.route(['C:\\\\show.sdc'],'[data-project-drop-surface]')");assert.equal(await fx("return f.calls.at(-1).name"),"openProject");
  await fx("f.route(['C:\\\\track.wav'],'[data-timeline-audio-layer-id]')");assert.deepEqual(await fx("return f.calls.at(-1)"),{name:"addAudio",args:[8,"C:\\track.wav"]});
  await fx("f.route(arg,'[data-video-clip-slot-layer-id]')",paths);assert.deepEqual(await fx("return f.calls.at(-1)"),{name:"assignSlots",args:[paths,3,4]});
  await fx("f.route(arg,'[data-internal]')",paths);assert.equal(await fx("return f.calls.at(-1).name"),"message","off-surface visual rejected");
  const detachedCount=await fx("return f.calls.length");
  await page.evaluate(()=>{const target=document.createElement("div");target.dataset.mediaLibraryRail="";target.dataset.detachedDrop="";document.body.append(target);});
  await drag("[data-detached-drop]");await page.evaluate(()=>document.querySelector("[data-detached-drop]").remove());await fx("f.reply(arg)",paths);
  assert.equal(await fx("return f.calls.length"),detachedCount,"removed targets cannot route");
  if(locale==="ja"&&width===1920) {
   await page.clock.install();await drag(library);await fx("f.advanceContext()");await page.clock.runFor(10_001);
   assert.equal(await fx("return f.calls.length"),detachedCount,"stale timeout cannot report into new project");
   await drag(library);await page.clock.runFor(10_001);assert.equal(await fx("return f.calls.at(-1).name"),"message","live timeout reports rejection");
  }
  const beforeStop=await fx("return f.calls.length");await drag(library);await fx("f.stop();f.reply(arg)",paths);assert.equal(await fx("return f.calls.length"),beforeStop,"disposed listener cannot route");
  assert.equal((await drag("[data-internal]","dragover")).prevented,false,"disposed dragover listener is removed");
  assert.deepEqual(errors,[],`${locale}-${width}: console/page health`);
  results.push({locale,width,height,url,identity:true,notBlank:true,noOverlay:true,consoleErrors:errors,hoverContainment:true,dropRouting:true,rejection:true,internalPayload:true,staleContext:true,teardown:true});
  await page.close();
 }
 await writeFile(path.join(evidence,"results.json"),JSON.stringify(results,null,2));
 console.log(`PASS native drop: ${results.length} locale/viewport cases; routing, rejection, hover, internal payloads, stale authority/replies, teardown. Evidence: ${evidence}`);
} finally {await browser?.close();await server.close();}
