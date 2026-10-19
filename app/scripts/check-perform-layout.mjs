// Browser plugin not available; use the repository's raw Chromium CDP approach.
// This browser fixture verifies layout/control routing, never native or physical output.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createServer as createTcpServer } from "node:net";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const evidenceDir = resolve(process.env.SYNDOCAL_PERFORM_SCREENSHOT_DIR ?? join(appRoot, "../target/qa/vj-perform-ui/browser"));
const browserPath = [process.env.CHROME_PATH,
  "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
  "C:/Program Files/Microsoft/Edge/Application/msedge.exe",
].filter(Boolean).find(existsSync);
assert.ok(browserPath, "Set CHROME_PATH to an installed Chromium browser");
const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
async function waitFor(check, label, timeout = 30_000) {
  const deadline = Date.now() + timeout;
  let lastError;
  while (Date.now() < deadline) {
    try { const result = await check(); if (result) return result; } catch (error) { lastError = error; }
    await sleep(75);
  }
  throw new Error(`Timed out: ${label}${lastError ? ` (${lastError.message})` : ""}`);
}

class Cdp {
  pending = new Map();
  nextId = 0;
  errors = [];
  dialogs = [];
  acceptDialogs = true;
  async connect(url) {
    this.socket = new WebSocket(url);
    await new Promise((done, reject) => {
      const timer = setTimeout(() => reject(new Error("CDP connection timed out")), 10_000);
      this.socket.addEventListener("open", () => { clearTimeout(timer); done(); }, { once: true });
      this.socket.addEventListener("error", () => { clearTimeout(timer); reject(new Error("CDP connection failed")); }, { once: true });
    });
    this.socket.addEventListener("message", ({ data }) => {
      const message = JSON.parse(data);
      if (message.method === "Runtime.exceptionThrown") this.errors.push(message.params.exceptionDetails);
      if (message.method === "Runtime.consoleAPICalled" && ["error", "warning"].includes(message.params.type)) this.errors.push(message.params);
      if (message.method === "Page.javascriptDialogOpening") { this.dialogs.push(message.params); void this.send("Page.handleJavaScriptDialog", { accept: this.acceptDialogs }); }
      const call = this.pending.get(message.id);
      if (!call) return;
      clearTimeout(call.timer);
      this.pending.delete(message.id);
      if (message.error) call.reject(new Error(`${call.method}: ${JSON.stringify(message.error)}`));
      else call.done(message.result);
    });
    this.socket.addEventListener("close", () => {
      for (const call of this.pending.values()) { clearTimeout(call.timer); call.reject(new Error("CDP disconnected")); }
      this.pending.clear();
    });
  }
  send(method, params = {}, timeout = 15_000) {
    return new Promise((done, reject) => {
      const id = ++this.nextId;
      const timer = setTimeout(() => { this.pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, timeout);
      this.pending.set(id, { done, reject, timer, method });
      this.socket.send(JSON.stringify({ id, method, params }));
    });
  }
  async evaluate(expression) {
    const result = await this.send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
    assert.ok(!result.exceptionDetails, JSON.stringify(result.exceptionDetails));
    return result.result.value;
  }
  // Headless Chromium occasionally stalls one screenshot (most often with a modal
  // dialog open); evidence capture retries instead of failing the layout contract.
  async capture(params = { format: "png" }, timeout = 30_000) {
    for (let attempt = 1; ; attempt++) {
      try { return await this.send("Page.captureScreenshot", params, timeout); }
      catch (error) {
        if (attempt >= 3 || !/CDP timeout/.test(error.message)) throw error;
        await this.send("Page.bringToFront").catch(() => {});
      }
    }
  }
  async key(key, code = key, virtualKey) {
    const params = { key, code, ...(virtualKey ? { windowsVirtualKeyCode: virtualKey, nativeVirtualKeyCode: virtualKey } : {}) };
    await this.send("Input.dispatchKeyEvent", { type: "keyDown", ...params, ...(key === "Enter" ? { text: "\r", unmodifiedText: "\r" } : {}) });
    await this.send("Input.dispatchKeyEvent", { type: "keyUp", ...params });
  }
}

const root = "[data-video-perform-layout]";
const selectors = {
  preview: `${root} [data-live-video-monitor='preview']`,
  program: `${root} [data-live-video-monitor='program']`,
  master: `${root} [data-video-perform-master]`,
  launch: `${root} [data-video-perform-row='1'] [data-video-perform-cell-index='0']`,
  take: `${root} [data-video-perform-take]`,
  transitionKind: `${root} [data-video-perform-transition-kind]`,
  transitionDuration: `${root} [data-video-perform-transition-duration]`,
  fxEditor: `${root} [data-video-perform-fx-editor]`,
  guardedBlackout: `${root} [data-video-perform-blackout]`,
  legend: `${root} [data-video-perform-legend-toggle]`,
  inspect: `${root} [data-video-perform-inspect]`,
  clipOptions: `${root} [data-video-perform-clip-options]`,
  blackout: "[data-global-operator-action='video-blackout']",
  // Tempo has one home: the global top bar cluster, visible in every workspace.
  tap: ".topbarTempoCluster [data-topbar-tap]",
  bpm: ".topbarTempoCluster .topbarBpmControl",
  sync: ".topbarTempoCluster [data-topbar-sync]",
  beats: ".topbarTempoCluster [data-topbar-sync] .topbarBeats",
};

// Intersect every scroll/clipping ancestor as well as the viewport. A center
// hit alone could pass a half-hidden control, so require its complete rectangle.
function inspectInPage(selector) {
  const element = typeof selector === "string" ? document.querySelector(selector) : selector;
  if (!element) return { selector, failure: "missing" };
  const rect = element.getBoundingClientRect();
  const style = getComputedStyle(element);
  let failure = rect.width <= 0 || rect.height <= 0 || style.visibility !== "visible" || style.display === "none" ? "hidden" : null;
  let clip = { left: 0, top: 0, right: innerWidth, bottom: innerHeight };
  for (let parent = element.parentElement; parent; parent = parent.parentElement) {
    const bounds = parent.getBoundingClientRect();
    const parentStyle = getComputedStyle(parent);
    if (/(auto|scroll|hidden|clip)/.test(parentStyle.overflowX)) { clip.left = Math.max(clip.left, bounds.left); clip.right = Math.min(clip.right, bounds.right); }
    if (/(auto|scroll|hidden|clip)/.test(parentStyle.overflowY)) { clip.top = Math.max(clip.top, bounds.top); clip.bottom = Math.min(clip.bottom, bounds.bottom); }
  }
  if (rect.left < clip.left - 1 || rect.top < clip.top - 1 || rect.right > clip.right + 1 || rect.bottom > clip.bottom + 1) failure ??= "clipped";
  const hit = document.elementFromPoint(rect.left + rect.width / 2, rect.top + rect.height / 2);
  if (!(hit === element || element.contains(hit))) failure ??= "covered";
  return { selector, failure, x: rect.x, y: rect.y, width: rect.width, height: rect.height, value: element.value, tabIndex: element.tabIndex, disabled: Boolean(element.disabled) };
}

// Mount the real shared bank with reactive inputs. The full browser fixture has
// no desktop polling, so it cannot catch focus loss when runtime rows refresh.
const focusFixturePath = "/src/__perform-focus-fixture.tsx";
const focusFixtureId = join(appRoot, focusFixturePath.slice(1)).replaceAll("\\", "/");
const focusFixtureSource = `
  import { createSignal } from "solid-js";
  import { render } from "solid-js/web";
  import { VideoClipSlotBankPanel } from "/src/components/VideoClipSlotBankPanel.tsx";
  const slots = [101, 102].map((id) => ({ id, media_asset_id: id, launch_quantization: "Immediate" }));
  const [layer, setLayer] = createSignal({ id: 1, label: "Focus fixture", clip_slots: slots });
  const [runtime, setRuntime] = createSignal({ layer_id: 1, active_slot_id: 101, queued_slot_id: null, pending_launch: null, transition: null, playhead_ms: 0 });
  const [selected, setSelected] = createSignal(101);
  const [performance, setPerformance] = createSignal(true);
  const queueCalls = [];
  Object.assign(window, { __performFocus: {
    refresh: () => setRuntime({ ...runtime(), playhead_ms: runtime().playhead_ms + 250 }),
    queue: () => setRuntime({ ...runtime(), active_slot_id: null, queued_slot_id: 101 }),
    replaceLayer: () => setLayer({ ...layer(), label: "Updated layer", clip_slots: layer().clip_slots.map(slot => ({ ...slot })) }),
    setPerformance, queueCalls,
  }});
  render(() => <VideoClipSlotBankPanel mode="control" performance={performance()} layers={[layer()]} layer={layer()} runtime={runtime()}
    assets={[]} thumbnails={{}} selectedSlotId={selected()} onSelectLayer={() => {}} onSelect={setSelected} onQueue={id => queueCalls.push(id)}
    onPreview={() => {}} onMore={() => {}} onImport={() => {}} onCancelQueue={() => {}} onSetQuantization={() => {}}
    transitionKind="Cut" transitionDurationUnit="Milliseconds" transitionDurationMilliunits={750}
    onSetTransitionKind={() => {}} onSetTransitionDurationUnit={() => {}} onSetTransitionDurationMilliunits={() => {}} />, document.getElementById("root"));
`;

const matrixFocusPath = "/src/__perform-matrix-focus-fixture.tsx";
const matrixFocusId = join(appRoot, matrixFocusPath.slice(1)).replaceAll("\\", "/");
const matrixFocusSource = `
 import { createSignal } from 'solid-js';
 import { render } from 'solid-js/web';
 import { VideoPerformMatrix } from '/src/components/VideoPerformMatrix.tsx';
 import { VideoEffectBrowser } from '/src/components/VideoEffectBrowser.tsx';
 import { InAppDialogHost } from '/src/components/InAppDialogHost.tsx';
 import { buildFxCatalog } from '/src/fxCatalogModel.ts';
 import { viewportFixtureData } from '/src/viewportFixtureData.ts';
 import '/src/theme.css';import '/src/styles.css';
 const initial=viewportFixtureData.vjBankLayers.slice(0,3).map((layer,i)=>({...structuredClone(layer),clip_slots:[101,102].map(id=>({id:i*100+id,media_asset_id:id,launch_quantization:'Immediate'}))}));
 const [layers,setLayers]=createSignal(initial);
 const [selectedLayer,setSelectedLayer]=createSignal(1),[selected,setSelected]=createSignal(101);
 const [runtime,setRuntime]=createSignal({layer_id:1,active_slot_id:101,queued_slot_id:null,pending_launch:null,transition:null,playhead_ms:0,playing:true});
 const [previewLayerId,setPreviewLayerId]=createSignal(null);
 const queueCalls=[],takes=[],inspectCalls=[],moreCalls=[],effectAdds=[];
 Object.assign(window,{__matrixFocus:{
 refresh:()=>setRuntime({...runtime(),playhead_ms:runtime().playhead_ms+250}),
 queue:()=>setRuntime({...runtime(),active_slot_id:null,queued_slot_id:101}),
 replaceLayer:()=>setLayers(layers().map(l=>({...l,label:l.id===1?'Updated matrix layer':l.label,clip_slots:l.clip_slots.map(s=>({...s}))}))),
 setPreviewLayerId,setRuntime,queueCalls,takes,inspectCalls,moreCalls,effectAdds,
 }});
 const bank={mode:'control',get layers(){return layers()},get layer(){return layers().find(l=>l.id===selectedLayer())},get runtime(){return runtime()},assets:[101,102].map(id=>({id,label:'Focus asset '+id,source:initial[0].source})),thumbnails:{},get selectedSlotId(){return selected()},onSelectLayer:setSelectedLayer,onSelect:setSelected,onQueue:id=>queueCalls.push([selectedLayer(),id]),onPreview:id=>inspectCalls.push(id),onMore:(id,trigger)=>moreCalls.push({layer:selectedLayer(),slot:id,selected:selected(),anchor:trigger.matches('[data-video-perform-cell-index]')?'cell':trigger.hasAttribute('data-video-perform-clip-options')?'corner':'other'}),onImport:()=>{},onCancelQueue:()=>{},onSetQuantization:()=>{},transitionKind:'Cut',transitionDurationUnit:'Milliseconds',transitionDurationMilliunits:750,onSetTransitionKind:()=>{},onSetTransitionDurationUnit:()=>{},onSetTransitionDurationMilliunits:()=>{}};
 const list={get layers(){return layers()},onSetLayerState:(id,state)=>setLayers(layers().map(l=>l.id===id?{...l,state}:l)),onSetBlendMode:(id,blend_mode)=>setLayers(layers().map(l=>l.id===id?{...l,blend_mode}:l))};
 render(()=><><InAppDialogHost locale={()=>'en'}/><div style={{height:'680px',width:'1300px'}}><VideoPerformMatrix bank={bank} layerList={list} effectEntries={buildFxCatalog()} onAddEffect={(entry,id)=>effectAdds.push([entry.id,id])} runtimes={[runtime()]} availabilityById={{}} previewLayerId={previewLayerId()} thumbnailsAuthorized={false} onStop={()=>{}} keyboardEnabled={true} onTake={()=>takes.push(true)}/></div><section data-video-perform-pane="inspector" style={{height:'300px',width:'600px'}}><VideoEffectBrowser entries={buildFxCatalog()} layer={layers()[1]} onAdd={(entry,id)=>effectAdds.push([entry.id,id])}/></section></>,document.getElementById('root'));
`;

const quickFxFocusPath = "/src/__perform-quick-fx-focus-fixture.tsx";
const quickFxFocusId = join(appRoot, quickFxFocusPath.slice(1)).replaceAll("\\", "/");
const quickFxFocusSource = `
 import { createSignal } from 'solid-js';
 import { render } from 'solid-js/web';
 import { VideoPerformQuickFx } from '/src/components/VideoPerformQuickFx.tsx';
 import '/src/theme.css';import '/src/styles.css';
 const control={name:'gain',kind:'Float',value:[0.5,0,0,0],default:[0.5,0,0,0],minimum:[0,0,0,0],maximum:[1,0,0,0],values:[],labels:[]};
 const [layer,setLayer]=createSignal({id:1,label:'Focus layer',isf_effect:{label:'Focus effect',enabled:true,controls:[control],stack:[]}});
 const calls=[];
 const replace=(value,enabled)=>setLayer({...structuredClone(layer()),label:'Updated FX layer',isf_effect:{...structuredClone(layer().isf_effect),enabled,controls:[{...structuredClone(control),value:[value,0,0,0]}]}});
 Object.assign(window,{__quickFxFocus:{
   refresh:()=>setLayer(structuredClone(layer())),
   replace,calls,
 }});
 const list={isfEventPulseBusy:false,
   onSetIsfEffectEnabled:(layerId,index,enabled)=>{calls.push(['enabled',layerId,index,enabled]);replace(layer().isf_effect.controls[0].value[0],enabled)},
   onSetIsfControl:(layerId,index,name,value)=>{calls.push(['control',layerId,index,name,value]);replace(value[0],layer().isf_effect.enabled)},
 };
 render(()=><div style={{height:'180px',width:'1000px'}}><VideoPerformQuickFx layer={layer()} layerList={list} onOpenEditor={()=>{}}/></div>,document.getElementById('root'));
`;

const focusFixturePlugin = {
  name: "perform-focus-regression-fixture",
  enforce: "pre",
  resolveId(id) { if (id === focusFixturePath) return focusFixtureId; if(id===matrixFocusPath)return matrixFocusId; if(id===quickFxFocusPath)return quickFxFocusId; },
  load(id) { if (id === focusFixtureId) return focusFixtureSource; if(id===matrixFocusId)return matrixFocusSource; if(id===quickFxFocusId)return quickFxFocusSource; },
  configureServer(instance) {
    instance.middlewares.use((request, response, next) => {
      if (!["/__perform-focus", "/__perform-matrix-focus", "/__perform-quick-fx-focus"].includes(request.url)) return next();
      response.setHeader("Content-Type", "text/html");
      response.end('<!doctype html><title>Perform focus regression</title><div id="root"></div><script type="module" src="' + (request.url === '/__perform-matrix-focus' ? matrixFocusPath : request.url === '/__perform-quick-fx-focus' ? quickFxFocusPath : focusFixturePath) + '"></script>');
    });
  },
};

let server;
let browser;
let cdp;
let profile;
const results = [];
let currentCase = "startup";
try {
  await mkdir(evidenceDir, { recursive: true });
  const reservation = createTcpServer();
  await new Promise((done, reject) => { reservation.once("error", reject); reservation.listen(0, "127.0.0.1", done); });
  const vitePort = reservation.address().port;
  await new Promise((done, reject) => reservation.close((error) => error ? reject(error) : done()));
  server = await createServer({ root: appRoot, cacheDir: resolve(evidenceDir, "vite-cache"), server: { host: "127.0.0.1", port: vitePort, strictPort: true }, clearScreen: false, plugins: [focusFixturePlugin] });
  await server.listen();
  await server.environments.client.depsOptimizer?.scanProcessing;
  await server.transformRequest("/src/App.tsx");
  const origin = server.resolvedUrls.local[0];
  profile = await mkdtemp(join(tmpdir(), "syndocal-perform-check-"));
  browser = spawn(browserPath, ["--headless=new", "--disable-gpu", "--disable-background-timer-throttling", "--disable-renderer-backgrounding", "--no-first-run", "--no-default-browser-check", "--remote-debugging-port=0", `--user-data-dir=${profile}`, "about:blank"], { windowsHide: true, stdio: "ignore" });
  browser.on("error", (error) => { console.error(error); process.exitCode = 1; });
  const port = await waitFor(async () => Number((await readFile(join(profile, "DevToolsActivePort"), "utf8")).split("\n")[0]), "owned browser debugger");
  const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  let pageTarget = targets.find((target) => target.type === "page");
  cdp = new Cdp();
  await cdp.connect(pageTarget.webSocketDebuggerUrl);
  await cdp.send("Page.enable");
  await cdp.send("Runtime.enable");
  const freshTarget = async () => {
    const nextTarget = await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: "PUT" })).json();
    await fetch(`http://127.0.0.1:${port}/json/close/${pageTarget.id}`);
    cdp.socket.close();
    cdp = new Cdp();
    pageTarget = nextTarget;
    await cdp.connect(pageTarget.webSocketDebuggerUrl);
    await cdp.send("Page.enable");
    await cdp.send("Runtime.enable");
    await cdp.send("Page.bringToFront");
  };
  for (const locale of (process.env.SYNDOCAL_PERFORM_FOCUS_ONLY === "1" ? [] : ["en", "ja"])) {
    for (const [width, height] of [[1920, 1080], [1366, 768], [1280, 720]]) {
      currentCase = `perform-${locale}-${width}x${height}`;
      if (results.length > 0) {
        // Each case owns a fresh target. Navigating a dirty Solid desk reuses
        // beforeunload/renderer state and can detach Chromium's screenshot host.
        await freshTarget();
      }
      const label = `perform-${locale}-${width}x${height}`;
      currentCase = label;
      const url = `${origin}?syndocalViewportFixture=vj-bank&performCase=${label}`;
      await cdp.send("Emulation.setDeviceMetricsOverride", { width, height, deviceScaleFactor: 1, mobile: false });
      await cdp.send("Page.addScriptToEvaluateOnNewDocument", { source: `localStorage.setItem('syndocal.uiLocale.v1', ${JSON.stringify(locale)}); localStorage.setItem('syndocal.workspaceLayout.v1', JSON.stringify({workspace_tab:'touch',control_mode:'perform',touch_control_domain:'video'}));` }).then(async ({ identifier }) => {
        await cdp.send("Page.navigate", { url });
        await waitFor(() => cdp.evaluate(`location.href === ${JSON.stringify(url)} && document.readyState === 'complete'`), `${label} navigation committed`);
        await cdp.send("Page.bringToFront");
        await waitFor(() => cdp.evaluate("!!document.querySelector('[data-workspace-option=touch]')"), `${label} workspace chrome`);
        // Existing vj-bank fixture intentionally opens its historic Mixer desk.
        // Enter the new surface through the actual workspace control.
        await cdp.evaluate("document.querySelector('[data-workspace-option=touch]').click(); document.querySelector('[data-control-domain=video]').click()");
        await waitFor(() => cdp.evaluate(`!!document.querySelector(${JSON.stringify(root)}) && !!document.querySelector(${JSON.stringify(selectors.launch)})`), `${label} loaded`);
        await cdp.send("Page.removeScriptToEvaluateOnNewDocument", { identifier });
      });
      await cdp.evaluate("document.fonts.ready.then(() => true)");
      const expectedHeading = locale === "ja" ? "ライブ映像" : "Perform";
      await waitFor(() => cdp.evaluate(`document.querySelector('${root} [data-video-perform-pane="matrix"] .workspacePaneTitle')?.textContent === ${JSON.stringify(expectedHeading)}`), `${label}: rendered locale`);
      if (locale === "ja") {
        for (const [state, translated] of [["Loaded", "読み込み済み"], ["Queued", "キュー中"]]) {
          await waitFor(() => cdp.evaluate(`Array.from(document.querySelectorAll('[data-video-perform-cell-state="${state}"] .videoPerformClipState')).some(e => e.textContent.includes(${JSON.stringify(translated)}))`), `${label}: dynamic ${state} state localized`);
        }
      }
      if (locale === "ja") {
        const performText = await cdp.evaluate(`document.querySelector(${JSON.stringify(root)}).innerText`);
        assert.doesNotMatch(performText, /\?{2,}|\uFFFD/, `${label}: Japanese text has no replacement runs`);
      }
      await cdp.evaluate("window.__performKeys=[]; window.__performClicks=[]; document.addEventListener('keydown', e => queueMicrotask(() => window.__performKeys.push({key:e.key,prevented:e.defaultPrevented,target:e.target.outerHTML.slice(0,240)}))); document.addEventListener('click', e => window.__performClicks.push(e.target.closest('[data-global-operator-action]')?.dataset.globalOperatorAction ?? 'other'))");
      const identity = await cdp.evaluate(`({url:location.href,title:document.title,text:document.body.innerText.length,overlay:!!document.querySelector('vite-error-overlay'),width:innerWidth,height:innerHeight,scrollWidth:document.documentElement.scrollWidth,scrollHeight:document.documentElement.scrollHeight})`);
      assert.equal(identity.url, url, label);
      assert.ok(identity.title && identity.text > 100, `${label}: meaningful application`);
      assert.equal(identity.overlay, false, `${label}: framework overlay`);
      assert.equal(identity.scrollWidth, width, `${label}: outer horizontal scroll`);
      assert.equal(identity.scrollHeight, height, `${label}: outer vertical scroll`);
      await writeFile(join(evidenceDir, `${label}-before.png`), Buffer.from((await cdp.capture( { format: "png", captureBeyondViewport: false, optimizeForSpeed: true }, 30_000)).data, "base64"));
      const inspect = (selector) => cdp.evaluate('('+inspectInPage+')('+JSON.stringify(selector)+')');
      const regions = ['matrix','transition','quick-fx','health','output'].map(name => root+' [data-video-perform-region="'+name+'"]');
      const required = [...Object.values(selectors), ...regions];
      const geometry = [];
      for (const selector of required) {
        const measurement = await inspect(selector); geometry.push(measurement);
        assert.equal(measurement.failure, null, label+': '+JSON.stringify(measurement));
        if (selector.startsWith(root) && !regions.includes(selector) && ![selectors.preview,selectors.program].includes(selector)) {
          assert.ok(measurement.height >= 44 && measurement.width >= 44, label+': preserved 44px control target '+selector);
        }
      }
      // Health lives in the desk header between the title and the shortcuts button and never below the matrix.
      const headerOrder = await cdp.evaluate('(()=>{const r=(s)=>document.querySelector(s)?.getBoundingClientRect();const h=r('+JSON.stringify(root+' .workspacePaneHeader [data-video-perform-region="health"]')+'),t=r('+JSON.stringify(root+' [data-video-perform-pane="matrix"] .workspacePaneTitle')+'),l=r('+JSON.stringify(root+' .workspacePaneHeader [data-video-perform-legend-toggle]')+');return h&&t&&l?{titleRight:t.right,healthLeft:h.left,healthRight:h.right,legendLeft:l.left,healthH:h.height,standalone:!!document.querySelector('+JSON.stringify(root+' > .videoPerformHealth')+')}:null})()');
      assert.ok(headerOrder, label+': health region is rendered inside the Perform pane header');
      assert.ok(headerOrder.healthLeft >= headerOrder.titleRight - 1 && headerOrder.healthRight <= headerOrder.legendLeft + 1, label+': health sits between title and shortcuts in the pane header '+JSON.stringify(headerOrder));
      assert.equal(headerOrder.standalone, false, label+': no separate bottom health row');
      // One tempo cluster: the Perform desk no longer carries its own BPM, Tap, sync or beat lights.
      const tempo = await cdp.evaluate('(()=>{const vis=(e)=>{const r=e.getBoundingClientRect();const s=getComputedStyle(e);return r.width>0&&r.height>0&&s.visibility==="visible"&&s.display!=="none"};const count=(q)=>[...document.querySelectorAll(q)].filter(vis).length;const rect=(q)=>{const r=document.querySelector(q)?.getBoundingClientRect();return r&&{left:r.left,right:r.right,top:r.top,bottom:r.bottom}};const sync=document.querySelector("[data-topbar-sync]");const dots=[...(sync?.querySelectorAll(".topbarBeats i")??[])];const lit=dots.map((d,i)=>d.classList.contains("lit")?i:-1).filter(i=>i>=0);const top=document.querySelector(".topbar");const fits=(q)=>{const e=document.querySelector(q);if(!e)return false;const range=document.createRange();range.selectNodeContents(e);return range.getBoundingClientRect().width<=e.getBoundingClientRect().width+1&&e.scrollWidth<=e.clientWidth+1};return {tap:count("[data-topbar-tap]"),tapTargets:count("[data-control-map-target*=\\\"TapBpm\\\"]"),bpm:count(".topbarBpmControl"),bpmTargets:count("[data-control-map-target*=\\\"SetBpm\\\"]"),deskTempo:document.querySelectorAll("[data-video-perform-layout] [data-video-perform-tap], [data-video-perform-layout] [data-video-perform-region=tempo], [data-video-perform-layout] .videoPerformTempo, [data-video-perform-layout] input[aria-label*=BPM]").length,tapWords:[...document.querySelectorAll("button")].filter(b=>vis(b)&&/^(Tap|タップ)$/.test(b.textContent.trim())).length,dots:dots.length,lit,beat:Number(sync?.querySelector(".topbarBeats")?.dataset.topbarBeat),source:sync?.querySelector(".topbarSyncSource")?.textContent.trim(),status:sync?.querySelector(".topbarSyncStatus")?.textContent.trim(),title:sync?.title,sourceFits:fits("[data-topbar-sync] .topbarSyncSource"),statusFits:fits("[data-topbar-sync] .topbarSyncStatus")&&fits("[data-topbar-sync] .topbarSyncRow"),cluster:rect(".topbarTempoCluster"),state:rect(".topbarStateCluster"),windowControls:rect("[data-window-controls]"),topbarOverflow:top.scrollWidth-top.clientWidth,viewportWidth:innerWidth}})()');
      assert.deepEqual([tempo.tap, tempo.tapTargets, tempo.tapWords, tempo.bpm, tempo.bpmTargets, tempo.deskTempo], [1, 1, 1, 1, 1, 0], label+': exactly one Tap and one BPM control, none on the Perform desk '+JSON.stringify(tempo));
      assert.equal(tempo.dots, 4, label+': four beat lights '+JSON.stringify(tempo));
      assert.deepEqual(tempo.lit, [tempo.beat % 4], label+': exactly the current beat is lit '+JSON.stringify(tempo));
      assert.ok(tempo.source && tempo.status && tempo.title?.includes(tempo.source) && tempo.title.includes(tempo.status), label+': sync source and status are shown and carried in title '+JSON.stringify(tempo));
      assert.ok(tempo.sourceFits && tempo.statusFits, label+': sync text is not truncated '+JSON.stringify(tempo));
      assert.ok(tempo.cluster.right <= tempo.state.left + 1 && tempo.state.right <= tempo.windowControls.left + 1 && tempo.windowControls.right <= tempo.viewportWidth + 1 && tempo.topbarOverflow <= 0, label+': top bar tempo cluster fits without overlap or overflow '+JSON.stringify(tempo));
      const rowSelector=root+' [data-video-perform-row="1"]';
      // Every Perform control is MIDI-learnable with stable ids: clip, Take, Stop, blend, transition type and time.
      const learnTargets = await cdp.evaluate('(()=>{const read=(q)=>{try{const t=JSON.parse(document.querySelector(q)?.dataset.controlMapTarget??"null");return t&&t[0]}catch{return null}};return {pad:read('+JSON.stringify(selectors.launch)+'),stop:read('+JSON.stringify(root+' [data-video-perform-row="1"] [data-video-layer-stop]')+'),blend:read('+JSON.stringify(root+' [data-video-perform-row="1"] [data-video-layer-blend]')+'),take:read('+JSON.stringify(selectors.take)+'),kind:read('+JSON.stringify(selectors.transitionKind)+'),time:read('+JSON.stringify(selectors.transitionDuration)+'),padSlotId:Number(document.querySelector('+JSON.stringify(selectors.launch)+')?.closest("[data-video-clip-slot-id]")?.dataset.videoClipSlotId)}})()');
      assert.deepEqual([learnTargets.pad?.action, learnTargets.pad?.layer_id, learnTargets.pad?.midi_only], ['VideoClipSlotTrigger', 1, true], label+': the pad is a MIDI target '+JSON.stringify(learnTargets));
      assert.ok(Number.isSafeInteger(learnTargets.pad?.clip_slot_id) && learnTargets.pad.clip_slot_id > 0 && learnTargets.pad.clip_slot_id === learnTargets.padSlotId, label+': the pad targets its stable clip slot id, not its column '+JSON.stringify(learnTargets));
      assert.deepEqual([learnTargets.stop?.action, learnTargets.stop?.layer_id], ['VideoLayerStop', 1], label+': Stop is a MIDI target');
      assert.deepEqual([learnTargets.blend?.action, learnTargets.blend?.layer_id], ['VideoLayerBlendMode', 1], label+': Blend is a MIDI target');
      assert.equal(learnTargets.take?.action, 'VideoTake', label+': Take is a MIDI target');
      assert.equal(learnTargets.kind?.action, 'VideoTransitionKind', label+': transition type is a MIDI target');
      assert.deepEqual([learnTargets.time?.action, learnTargets.time?.low, learnTargets.time?.high], ['VideoTransitionTime', 0.1, 10], label+': transition time is a MIDI target');
      const legendText = await cdp.evaluate('document.querySelector('+JSON.stringify(root+' [data-video-perform-legend]')+').textContent');
      assert.doesNotMatch(legendText, /not supported by the current control model|現在の制御モデルは/, label+': the legend no longer reports unsupported MIDI targets');
      // Density: complete layer rows inside the matrix viewport (excluding the sticky head and horizontal scrollbar).
      const matrixDensity = await cdp.evaluate('(()=>{const sc=document.querySelector('+JSON.stringify(root+' .videoPerformMatrixScroll')+');sc.scrollTop=0;const b=sc.getBoundingClientRect();const head=sc.querySelector(".videoPerformMatrixHead").getBoundingClientRect();const limit=b.top+sc.clientTop+sc.clientHeight;const rows=[...sc.querySelectorAll("[data-video-perform-row]")];const full=rows.filter(r=>{const x=r.getBoundingClientRect();return x.top>=head.bottom-1&&x.bottom<=limit+1});const first=rows[0].getBoundingClientRect();const cell=rows[0].querySelector(".videoPerformMatrixCell");return {fullRows:full.length,totalRows:rows.length,headHeight:head.height,rowHeight:first.height,stripHeight:rows[0].querySelector(".videoPerformLayerStrip").getBoundingClientRect().height,cellButtons:cell.querySelectorAll("button").length,actionRows:document.querySelectorAll(".videoPerformClipActions").length,numberBadges:document.querySelectorAll(".videoPerformClipNumber").length,keyBadges:cell.querySelectorAll("kbd").length}})()');
      const expectedRows = width >= 1920 ? 4 : 3;
      assert.ok(matrixDensity.totalRows >= 4, label+': fixture has at least 4 layers '+JSON.stringify(matrixDensity));
      assert.ok(matrixDensity.fullRows >= expectedRows, label+': at least '+expectedRows+' complete layer rows visible '+JSON.stringify(matrixDensity));
      assert.equal(matrixDensity.headHeight, 44, label+': matrix head is 44px');
      assert.ok(matrixDensity.rowHeight <= 106 && matrixDensity.stripHeight <= 104, label+': compact layer row '+JSON.stringify(matrixDensity));
      assert.deepEqual([matrixDensity.cellButtons, matrixDensity.actionRows, matrixDensity.numberBadges, matrixDensity.keyBadges], [1, 0, 0, 1], label+': clip cell is the trigger only (key badge kept, no number badge or action row)');
      const visibleColumns=await cdp.evaluate('Array.from(document.querySelectorAll('+JSON.stringify(rowSelector+' [data-video-perform-cell-index]')+')).filter(e=>('+inspectInPage+')(e).failure===null).map(e=>Number(e.dataset.videoPerformCellIndex))');
      assert.ok(visibleColumns.length >= (width>=1920?15:width>=1366?8:7), label+': fully visible clip columns '+visibleColumns.length);
      if(width===1920)assert.ok(visibleColumns.length>8,label+': wide display exposes more than 8 columns');
      assert.ok(visibleColumns.every((x,i)=>x===i),label+': first clip columns visible');
      const stripParts = {};
      for (const [part, selector] of Object.entries({ name: '[data-video-perform-select-layer="1"]', live: '[data-video-perform-layer-live]', solo: '[data-video-layer-solo="1"]', bypass: '[data-video-layer-bypass="1"]', stop: '[data-video-layer-stop="1"]', opacity: '[data-video-layer-opacity="1"]', blend: '[data-video-layer-blend="1"]' })) {
        const measurement = await inspect(rowSelector+' '+selector); geometry.push(measurement);
        assert.equal(measurement.failure, null, label+': strip '+part+' '+JSON.stringify(measurement));
        if (part !== 'live') assert.ok(measurement.height >= 44 && measurement.width >= 44, label+': strip hit target '+part+' '+JSON.stringify(measurement));
        stripParts[part] = measurement;
      }
      assert.ok(stripParts.name.width >= 140, label+': layer name keeps room '+stripParts.name.width);
      assert.ok(Math.abs(stripParts.name.y - stripParts.solo.y) <= 2 && Math.abs(stripParts.solo.y - stripParts.bypass.y) <= 2 && Math.abs(stripParts.solo.y - stripParts.stop.y) <= 2, label+': name, Solo, Bypass and Stop share strip row 1');
      assert.ok(stripParts.opacity.y >= stripParts.solo.y + 44 && Math.abs(stripParts.opacity.y - stripParts.blend.y) <= 2 && Math.abs(stripParts.live.y + stripParts.live.height / 2 - stripParts.opacity.y - stripParts.opacity.height / 2) <= 6, label+': live state, opacity and blend share strip row 2');
      assert.ok(stripParts.live.width <= 72 && stripParts.live.x < stripParts.opacity.x, label+': live state is a compact leading item '+stripParts.live.width);
      const liveFit = await cdp.evaluate('(()=>{const e=document.querySelector('+JSON.stringify(rowSelector+' [data-video-perform-layer-live]')+');return {fits:e.scrollWidth<=e.clientWidth+1,title:e.title}})()');
      assert.ok(liveFit.fits && liveFit.title, label+': live state word is fully visible and carried in title '+JSON.stringify(liveFit));
      const liveText = await cdp.evaluate('document.querySelector('+JSON.stringify(rowSelector+' [data-video-perform-layer-live]')+').textContent.trim()');
      assert.match(liveText, /^(▶|❚❚|■)\s*\S+/, label+': live indicator has a symbol and text, not colour alone: '+liveText);
      const stoppedText = await cdp.evaluate('document.querySelector('+JSON.stringify(root+' [data-video-perform-row="2"] [data-video-perform-layer-live]')+').textContent.trim()');
      assert.equal(stoppedText, locale === 'ja' ? '■停止中' : '■Stopped', label+': stopped layer state word is distinct from the Stop button');
      assert.match(liveText, locale === 'ja' ? /LIVE$/ : /LIVE$/, label+': live state keeps LIVE');
      assert.equal(await cdp.evaluate('document.querySelector('+JSON.stringify(rowSelector+' [data-video-layer-stop]')+').textContent.trim()'), locale === 'ja' ? '停止' : 'Stop', label+': Stop button label unchanged');
      for(const attr of ['opacity','blend']){
        const selector=rowSelector+' [data-video-layer-'+attr+']';const measurement=await inspect(selector);geometry.push(measurement);
        assert.equal(measurement.failure,null,label+': strip '+attr);assert.ok(measurement.height>=44&&measurement.width>=44,label+': strip hit target '+attr);
      }
      // ---- Top 1 / bottom 2: Perform shares the Lighting arrangement. ----
      const paneRect = (name) => cdp.evaluate('(()=>{const e=document.querySelector('+JSON.stringify(root+' [data-video-perform-pane="'+name+'"]')+');if(!e)return null;const r=e.getBoundingClientRect();return {left:r.left,top:r.top,right:r.right,bottom:r.bottom,width:r.width,height:r.height}})()');
      const panes = { matrix: await paneRect('matrix'), output: await paneRect('output'), inspector: await paneRect('inspector') };
      for (const [name, rect] of Object.entries(panes)) {
        assert.ok(rect && rect.width > 100 && rect.height > 100, label+': '+name+' pane is visible '+JSON.stringify(rect));
        assert.ok(rect.left >= -1 && rect.top >= -1 && rect.right <= width + 1 && rect.bottom <= height + 1, label+': '+name+' pane stays inside the window '+JSON.stringify(rect));
      }
      assert.ok(panes.matrix.bottom <= panes.output.top + 8 && panes.matrix.bottom <= panes.inspector.top + 8, label+': matrix pane is above the output and inspector panes');
      assert.ok(panes.output.right <= panes.inspector.left + 8 && Math.abs(panes.output.top - panes.inspector.top) <= 1 && Math.abs(panes.output.bottom - panes.inspector.bottom) <= 1, label+': output (left) and inspector (right) share the bottom band');
      assert.ok(Math.abs(panes.matrix.width - (panes.output.width + panes.inspector.width)) <= 12, label+': the top pane spans the full width');
      assert.ok(panes.matrix.height / (panes.matrix.height + panes.output.height) >= 0.55 && panes.matrix.height / (panes.matrix.height + panes.output.height) <= 0.7, label+': default top/bottom split keeps the matrix at about 55-60% (clamped up to keep 3 layer rows) '+JSON.stringify(panes));
      const inside = (region, pane) => cdp.evaluate('(()=>{const e=document.querySelector('+JSON.stringify(region)+');const p=document.querySelector('+JSON.stringify(root+' [data-video-perform-pane="'+pane+'"]')+');const a=e.getBoundingClientRect(),b=p.getBoundingClientRect();return a.left>=b.left-1&&a.right<=b.right+1&&a.top>=b.top-1&&a.bottom<=b.bottom+1})()');
      for (const [region, pane] of [['matrix', 'matrix'], ['health', 'matrix'], ['transition', 'output'], ['quick-fx', 'inspector']]) {
        assert.equal(await inside(root+' [data-video-perform-region="'+region+'"]', pane), true, label+': '+region+' region lies inside the '+pane+' pane');
      }
      for (const selector of [selectors.take, selectors.guardedBlackout, selectors.master, selectors.transitionKind, selectors.transitionDuration]) {
        assert.equal(await inside(selector, 'output'), true, label+': '+selector+' lies inside the Output pane');
      }
      assert.equal(await inside(selectors.fxEditor, 'inspector'), true, label+': Full FX editor lies inside the Inspector pane');
      const monitors = await cdp.evaluate('(()=>{const p=document.querySelector('+JSON.stringify(selectors.preview)+').getBoundingClientRect(),g=document.querySelector('+JSON.stringify(selectors.program)+').getBoundingClientRect();return {previewW:p.width,previewH:p.height,programW:g.width,programH:g.height,side:g.left>=p.right-1&&Math.abs(g.top-p.top)<=1}})()');
      assert.equal(monitors.side, true, label+': Preview and Program sit side by side '+JSON.stringify(monitors));
      assert.ok(Math.abs(monitors.previewW / monitors.previewH - 16 / 9) < 0.05 && Math.abs(monitors.programW / monitors.programH - 16 / 9) < 0.05, label+': monitors are 16:9 '+JSON.stringify(monitors));
      assert.ok(monitors.previewW >= (width === 1366 ? 296 : width >= 1920 ? 400 : 240) && monitors.programW >= (width === 1366 ? 296 : width >= 1920 ? 400 : 240), label+': monitors are large '+JSON.stringify(monitors));
      // Matrix pane header: title, health and shortcuts; the removed desk header and 66px Quick FX strip are gone.
      assert.equal(await cdp.evaluate('document.querySelectorAll('+JSON.stringify(root+' .videoPerformDeskHeader, '+root+' .videoPerformQuickFxHeader strong')+').length'), 0, label+': legacy desk header and Quick FX strip are removed');
      // ---- Inspector: FX tab shows every stage toggle and its parameters. ----
      const fxTab = await cdp.evaluate('(()=>{const q=document.querySelector('+JSON.stringify(root+' [data-video-perform-region="quick-fx"]')+');const stage=q.querySelector("[data-perform-fx-stage]");const bypass=q.querySelector("[data-perform-fx-bypass]");const control=q.querySelector("[data-perform-fx-control]");return {selected:document.querySelector("[data-video-perform-inspector-tab=fx]").getAttribute("aria-selected"),stages:q.querySelectorAll("[data-perform-fx-stage]").length,bypass:!!bypass,bypassPressed:bypass?.getAttribute("aria-pressed"),stageHeight:stage?.getBoundingClientRect().height,controls:q.querySelectorAll("[data-perform-fx-control]").length,value:q.querySelector("[data-perform-fx-control] output")?.textContent??null,rangeHeight:control?.querySelector("input")?.getBoundingClientRect().height??null}})()');
      assert.equal(fxTab.selected, 'true', label+': FX is the default Inspector tab '+JSON.stringify(fxTab));
      assert.ok(fxTab.stages >= 1 && fxTab.bypass && fxTab.controls >= 1, label+': FX tab shows the stage toggles and parameters '+JSON.stringify(fxTab));
      assert.ok(fxTab.stageHeight >= 44 && fxTab.rangeHeight >= 44, label+': FX stage rows and parameter controls keep 44px targets '+JSON.stringify(fxTab));
      assert.ok(fxTab.value && fxTab.value.length > 0, label+': FX parameters show their value '+JSON.stringify(fxTab));
      assert.equal((await inspect(root+' [data-perform-fx-stage="0"] [data-perform-fx-bypass]')).failure, null, label+': stage toggle fully visible');
      assert.equal((await inspect(root+' [data-perform-fx-control] input')).failure, null, label+': first parameter fully visible');
      // ---- Inspector tabs switch; the Clip tab embeds the existing Clip Inspector without a modal. ----
      const tabState = () => cdp.evaluate('({selected:[...document.querySelectorAll("[data-video-perform-inspector-tab]")].filter(t=>t.getAttribute("aria-selected")==="true").map(t=>t.dataset.videoPerformInspectorTab),panel:document.querySelector("[data-video-perform-inspector-panel]")?.dataset.videoPerformInspectorPanel,clip:!!document.querySelector("[data-video-clip-slot-inspector-embedded]"),clipDialog:!!document.querySelector("[data-video-clip-slot-inspector][role=dialog]"),browser:!!document.querySelector("[data-video-perform-effect-browser]"),fx:!!document.querySelector("[data-perform-quick-fx]")})');
      const clickTab = (name) => cdp.evaluate('document.querySelector("[data-video-perform-inspector-tab='+name+']").click()');
      await clickTab('clip');
      assert.deepEqual(await tabState(), { selected: ['clip'], panel: 'clip', clip: true, clipDialog: false, browser: false, fx: false }, label+': Clip tab embeds the Clip Inspector (no dialog)');
      assert.equal((await inspect(root+' [data-video-clip-slot-inspector-embedded] > header')).failure, null, label+': embedded Clip Inspector header is visible (the body scrolls inside the pane)');
      await clickTab('browser');
      assert.deepEqual(await tabState(), { selected: ['browser'], panel: 'browser', clip: false, clipDialog: false, browser: true, fx: false }, label+': Browser tab mounts the FX catalog');
      const browserState = await cdp.evaluate(`(()=>{const cards=[...document.querySelectorAll('[data-fx-catalog-card]')],target=document.querySelector('[data-fx-add-target]'),card=cards[0],b=card.getBoundingClientRect(),pane=document.querySelector('[data-video-perform-inspector-panel]').getBoundingClientRect();return {cards:cards.length,target:target.textContent,live:!!document.querySelector('[data-fx-live-warning]'),visibleCardPixels:Math.min(b.bottom,pane.bottom)-Math.max(b.top,pane.top),fullyVisibleCards:cards.filter(c=>{const r=c.getBoundingClientRect();return r.top>=pane.top-1&&r.bottom<=pane.bottom+1}).length,textSize:parseFloat(getComputedStyle(card).fontSize),outer:[document.documentElement.scrollWidth,document.documentElement.scrollHeight]}})()`);
      assert.ok(browserState.cards >= 14, label+': all built-ins are cards');
      assert.equal(await cdp.evaluate("document.querySelectorAll('[data-fx-catalog-audition]').length"), browserState.cards, label+': every card offers the Preview-only Try beside the LIVE Add');
      assert.match(browserState.target, /1/); assert.ok(browserState.target.length > 14, label+': explicit layer add target');
      assert.equal(browserState.live, true, label+': explicit LIVE boundary');
      // The task's Browser viewports show >=44px of the first card. The
      // additional 720px regression case may show a partial card; scroll its
      // real Add control into view and verify the complete 44px target below.
      assert.ok(browserState.visibleCardPixels >= (height >= 768 ? 44 : 1), label+': card visible before internal scrolling '+JSON.stringify(browserState));
      // Task 14 addendum: the Inspector is a compact list row in narrow panes, so at least two effects are
      // fully visible without scrolling at the 1366x768 and larger viewports.
      assert.ok(browserState.fullyVisibleCards >= (height >= 768 ? 2 : 1), label+': at least two effect cards are fully visible '+JSON.stringify(browserState));
      assert.ok(browserState.textSize >= 14, label+': readable catalog text');
      assert.deepEqual(browserState.outer, [width,height], label+': browser never creates document scroll');
      const search = '[data-fx-catalog-search]';
      assert.equal((await inspect(search)).failure, null, label+': search fully visible');
      await cdp.evaluate(`(()=>{const e=document.querySelector('${search}');e.focus();e.value='monochrome';e.dispatchEvent(new Event('input',{bubbles:true}));})()`);
      await waitFor(()=>cdp.evaluate("document.querySelectorAll('[data-fx-catalog-card]').length===1"),label+': name search filters cards');
      assert.equal(await cdp.evaluate("document.querySelector('[data-fx-catalog-card]').dataset.fxCatalogCard"),'builtin:monochrome');
      await cdp.evaluate(`(()=>{const e=document.querySelector('${search}');e.value='';e.dispatchEvent(new Event('input',{bubbles:true}));document.querySelector('[data-fx-filter-toggle]').click();})()`);
      await cdp.evaluate("document.querySelector('[data-fx-category=Colour]').click()");
      assert.ok(await cdp.evaluate("[...document.querySelectorAll('[data-fx-catalog-card]')].every(e=>['builtin:invert','builtin:monochrome','builtin:colorize'].includes(e.dataset.fxCatalogCard))"),label+': category chips filter');
      await cdp.evaluate("document.querySelector('[data-fx-filter-toggle]').click();document.querySelector('[data-fx-category=All]').click();document.querySelector('[data-fx-catalog-search]').focus()");
      await cdp.key('Tab','Tab',9);
      assert.equal(await cdp.evaluate("document.activeElement.hasAttribute('data-fx-filter-toggle')"),true,label+': filter keyboard reachable');
      // Task 14: Import ISF and FX Chain now sit in the tool row (search, filter, import, chain), so the first card
      // follows them in tab order. Every stop in between is one of those tool-row buttons.
      let toolStops = 0;
      for (let attempt = 0; attempt < 4; attempt++) {
        await cdp.key('Tab','Tab',9);
        if (await cdp.evaluate("document.activeElement.hasAttribute('data-fx-catalog-card')")) break;
        assert.equal(await cdp.evaluate("!!document.activeElement.closest('.videoFxCatalogActions')"),true,label+': tab order between the filter and the first card is the tool-row actions');
        toolStops++;
      }
      assert.ok(toolStops <= 2,label+': at most Import ISF and FX Chain sit between the filter and the first card');
      assert.equal(await cdp.evaluate("document.activeElement.hasAttribute('data-fx-catalog-card')"),true,label+': card keyboard reachable');
      await cdp.evaluate("document.querySelector('[data-fx-catalog-add]').scrollIntoView({block:'nearest'})");
      const addTarget = await inspect('[data-fx-catalog-add]');
      assert.equal(addTarget.failure,null,label+': complete Add target is reachable by internal scrolling');
      assert.ok(addTarget.height>=44 && addTarget.width>=44,label+': Add keeps its 44px target');
      for (const theme of ['tally','cobalt']) {
        await cdp.evaluate(`document.documentElement.dataset.theme=${JSON.stringify(theme)};document.querySelector('[data-effect-browser]').scrollTop=0`);
        await writeFile(join(evidenceDir, `${label}-browser-${theme}.png`), Buffer.from((await cdp.capture({format:'png',captureBeyondViewport:false})).data,'base64'));
      }
      await cdp.evaluate("document.documentElement.dataset.theme='tally'");
      await clickTab('fx');
      assert.deepEqual(await tabState(), { selected: ['fx'], panel: 'fx', clip: false, clipDialog: false, browser: false, fx: true }, label+': FX tab returns');
      await cdp.evaluate("document.querySelector('[data-video-perform-inspector-tab=fx]').focus()");
      await cdp.key('ArrowRight','ArrowRight',39);
      assert.equal((await tabState()).panel,'clip',label+': Inspector arrow key selects Clip without matrix interception');
      assert.equal(await cdp.evaluate("document.activeElement.dataset.videoPerformInspectorTab"),'clip');
      await clickTab('fx');
      // Clip… (corner) and right-click open the options in the Clip tab, never a popover.
      await cdp.evaluate('document.querySelector('+JSON.stringify(selectors.clipOptions)+').click()');
      assert.deepEqual(await tabState(), { selected: ['clip'], panel: 'clip', clip: true, clipDialog: false, browser: false, fx: false }, label+': Clip… opens the Inspector Clip tab');
      assert.equal(await cdp.evaluate('document.querySelectorAll("dialog[open],[role=dialog]").length'), 0, label+': no popover or dialog was opened for clip options');
      await clickTab('fx');
      // ---- Dividers: drag changes the ratio, persists, double-click resets, keyboard resizes. ----
      const stored = () => cdp.evaluate('(()=>{const v=JSON.parse(localStorage.getItem("syndocal.workspaceLayout.v1")||"{}");return {top:v.perform_top_split_ratio,lower:v.perform_lower_split_ratio}})()');
      const mouse = (type, x, y, extra = {}) => cdp.send('Input.dispatchMouseEvent', { type, x, y, button: 'left', buttons: type === 'mouseReleased' ? 0 : 1, clickCount: 1, ...extra });
      const handleCenter = async (kind) => { const r = await cdp.evaluate('(()=>{const r=document.querySelector("[data-video-perform-layout] [data-workspace-splitter=\\"'+kind+'\\"]").getBoundingClientRect();return {x:r.left+r.width/2,y:r.top+r.height/2}})()'); return r; };
      const aria = (kind) => cdp.evaluate('Number(document.querySelector("[data-video-perform-layout] [data-workspace-splitter=\\"'+kind+'\\"]").getAttribute("aria-valuenow"))');
      const drag = async (kind, dx, dy) => { const c = await handleCenter(kind); await mouse('mouseMoved', c.x, c.y, { buttons: 0 }); await mouse('mousePressed', c.x, c.y); for (let i = 1; i <= 6; i++) await mouse('mouseMoved', c.x + dx * i / 6, c.y + dy * i / 6); await mouse('mouseReleased', c.x + dx, c.y + dy); };
      const dblclick = async (kind) => { const c = await handleCenter(kind); await mouse('mouseMoved', c.x, c.y, { buttons: 0 }); await mouse('mousePressed', c.x, c.y); await mouse('mouseReleased', c.x, c.y); await mouse('mousePressed', c.x, c.y, { clickCount: 2 }); await mouse('mouseReleased', c.x, c.y, { clickCount: 2 }); };
      const defaults = { top: 0.58, lower: 0.67 };
      await waitFor(async () => (await stored()).top !== undefined, label+': Perform ratios reach layout storage');
      const before = { stored: await stored(), topAria: await aria('upper-lower'), lowerAria: await aria('lower-left-right') };
      assert.equal(before.stored.top, defaults.top, label+': persisted top ratio starts at the default '+JSON.stringify(before));
      assert.equal(before.stored.lower, defaults.lower, label+': persisted lower ratio starts at the default '+JSON.stringify(before));
      await drag('upper-lower', 0, 24);
      await waitFor(async () => (await stored()).top > defaults.top, label+': horizontal divider drag persists a larger top ratio');
      assert.ok(await aria('upper-lower') > before.topAria, label+': horizontal divider aria value follows the drag');
      await drag('lower-left-right', -60, 0);
      await waitFor(async () => (await stored()).lower < defaults.lower, label+': vertical divider drag persists a smaller output ratio');
      assert.ok(await aria('lower-left-right') < before.lowerAria, label+': vertical divider aria value follows the drag');
      const dragged = await stored();
      assert.ok((await inspect(selectors.take)).failure === null && (await inspect(selectors.guardedBlackout)).failure === null, label+': Take and Blackout stay visible after resizing');
      await dblclick('upper-lower');
      await waitFor(async () => (await stored()).top === defaults.top, label+': double-click resets the horizontal divider');
      await dblclick('lower-left-right');
      await waitFor(async () => (await stored()).lower === defaults.lower, label+': double-click resets the vertical divider');
      await cdp.evaluate('document.querySelector("[data-video-perform-layout] [data-workspace-splitter=\\"upper-lower\\"]").focus()');
      await cdp.key('ArrowDown', 'ArrowDown', 40);
      await waitFor(async () => (await stored()).top > defaults.top, label+': keyboard resizes the horizontal divider');
      await dblclick('upper-lower');
      await waitFor(async () => (await stored()).top === defaults.top, label+': divider reset after keyboard resize');
      assert.equal(await cdp.evaluate('document.activeElement?.closest("[data-video-perform-layout]") !== null'), true, label+': divider keyboard focus stays in the desk');
      // ---- Collapse and maximise from the pane header icons (session state, panes stay mounted). ----
      const paneState = () => cdp.evaluate('(()=>{const r=(n)=>{const e=document.querySelector("[data-video-perform-pane=\\""+n+"\\"]");const b=e.getBoundingClientRect();return {w:Math.round(b.width),h:Math.round(b.height)}};const l=document.querySelector("[data-video-perform-layout]");return {matrix:r("matrix"),output:r("output"),inspector:r("inspector"),focus:l.dataset.performFocus??null,collapsed:l.dataset.performBottomCollapsed??null,takeMounted:!!document.querySelector("[data-video-perform-take]")}})()');
      let lastPane, stablePaneSamples = 0;
      await waitFor(async () => { const state=await paneState(),text=JSON.stringify(state);stablePaneSamples=text===lastPane?stablePaneSamples+1:0;lastPane=text;return stablePaneSamples>=3; },label+': divider ResizeObserver settles before reference geometry');
      const normal = await paneState();
      const restored = async (message) => { await waitFor(async () => JSON.stringify(await paneState()) === JSON.stringify(normal), message+' expected='+JSON.stringify(normal)); };
      await cdp.evaluate('document.querySelector("[data-video-perform-maximise=matrix]").click()');
      let state = await paneState();
      assert.ok(state.focus === 'matrix' && state.output.h === 0 && state.inspector.h === 0 && state.matrix.h > normal.matrix.h + 100 && state.takeMounted, label+': maximising the matrix hides the bottom band '+JSON.stringify(state));
      assert.equal(await cdp.evaluate('document.querySelector("[data-video-perform-maximise=matrix]").getAttribute("aria-pressed")'), 'true', label+': maximise reports its state');
      await cdp.evaluate('document.querySelector("[data-video-perform-maximise=matrix]").click()');
      await restored(label+': restoring brings back the exact pane sizes');
      await cdp.evaluate('document.querySelector("[data-video-perform-collapse=output]").click()');
      await waitFor(async () => (await paneState()).collapsed === 'true' && (await paneState()).output.h <= 48, label+': bottom panes collapsed');
      state = await paneState();
      assert.ok(state.collapsed === 'true' && state.output.h <= 48 && state.inspector.h <= 48 && state.matrix.h > normal.matrix.h + 100, label+': collapsing the bottom panes leaves only their headers '+JSON.stringify(state));
      assert.equal(await cdp.evaluate('document.querySelector("[data-video-perform-collapse=inspector]").getAttribute("aria-pressed")'), 'true', label+': collapse reports its state on both bottom panes');
      await cdp.evaluate('document.querySelector("[data-video-perform-collapse=inspector]").click()');
      await restored(label+': expanding restores the bottom panes');
      await cdp.evaluate('document.querySelector("[data-video-perform-maximise=output]").click()');
      await waitFor(async () => (await paneState()).output.w > normal.output.w, label+': Output maximised');
      state = await paneState();
      assert.ok(state.focus === 'output' && state.matrix.h === 0 && state.inspector.w === 0 && state.output.w > normal.output.w, label+': maximising Output gives it the workspace '+JSON.stringify(state));
      assert.equal((await inspect(selectors.take)).failure, null, label+': Take is visible with Output maximised');
      await cdp.evaluate('document.querySelector("[data-video-perform-maximise=output]").click()');
      await restored(label+': Output restored before the next pane is maximised');
      await cdp.evaluate('document.querySelector("[data-video-perform-maximise=inspector]").click()');
      await waitFor(async () => (await paneState()).inspector.w > normal.inspector.w, label+': Inspector maximised');
      state = await paneState();
      assert.ok(state.focus === 'inspector' && state.matrix.h === 0 && state.output.w === 0 && state.inspector.w > normal.inspector.w, label+': maximising Inspector gives it the workspace '+JSON.stringify(state));
      await cdp.evaluate('document.querySelector("[data-video-perform-maximise=inspector]").click()');
      await restored(label+': every pane restored after maximise/collapse');
      const focus = (selector) => cdp.evaluate('document.querySelector('+JSON.stringify(selector)+').focus({preventScroll:true})');
      // Real Tab traversal catches hidden tab stops, focus traps and omitted controls.
      await cdp.evaluate('document.activeElement?.blur()');
      const tabCandidates=[root+' [data-video-perform-cell-index][tabindex="0"]',selectors.master,selectors.take,selectors.tap,selectors.guardedBlackout,selectors.legend,selectors.inspect,selectors.clipOptions,rowSelector+' [data-video-layer-opacity]',rowSelector+' [data-video-layer-blend]'];
      const tabTargets=[];for(const selector of tabCandidates)if(!(await inspect(selector)).disabled)tabTargets.push(selector);
      const reached=new Set();
      for(let attempt=0;attempt<220&&reached.size<tabTargets.length;attempt++){
        await cdp.key('Tab','Tab',9);
        const matches=await cdp.evaluate(JSON.stringify(tabTargets)+'.filter(s=>document.activeElement?.matches(s))');matches.forEach(m=>reached.add(m));
      }
      assert.deepEqual([...reached].sort(),[...tabTargets].sort(),label+': core controls reachable by Tab');
      await focus(selectors.launch);await cdp.key('End','End',35);
      assert.equal(await cdp.evaluate('document.activeElement?.dataset.videoPerformCellIndex'),'31',label+': End reaches slot32');
      assert.equal((await inspect(rowSelector+' [data-video-perform-cell-index="31"]')).failure,null,label+': slot32 scrolls inside matrix');
      await cdp.key('Home','Home',36);
      assert.equal(await cdp.evaluate('document.activeElement?.dataset.videoPerformCellIndex'),'0',label+': Home returns slot1');
      await cdp.key('ArrowRight','ArrowRight',39);
      assert.equal(await cdp.evaluate('document.activeElement?.dataset.videoPerformCellIndex'),'1',label+': Right selects next clip');
      await cdp.key('ArrowDown','ArrowDown',40);
      assert.equal(await cdp.evaluate('document.activeElement?.dataset.videoPerformLayerId'),'2',label+': Down selects next layer');
      await cdp.key('ArrowUp','ArrowUp',38);
      assert.equal(await cdp.evaluate('document.activeElement?.dataset.videoPerformLayerId'),'1',label+': Up returns layer');
      // Numeric trigger invokes the actual existing queue command. Browser backend
      // refusal is expected and must not be mistaken for published native state.
      await cdp.key('1','Digit1',49);
      await waitFor(()=>cdp.evaluate("document.body.innerText.includes('Syndocal desktop backend is not connected in this browser preview.')"),label+': number-key queues through desktop-required route');
      if(!(await inspect(selectors.take)).disabled){await focus(selectors.take);await cdp.key('Enter','Enter',13);}
      const opacity=rowSelector+' [data-video-layer-opacity]',blend=rowSelector+' [data-video-layer-blend]';
      const beforeOpacity=(await inspect(opacity)).value;await focus(opacity);await cdp.key('ArrowLeft','ArrowLeft',37);
      assert.notEqual((await inspect(opacity)).value,beforeOpacity,label+': opacity keyboard changes');
      const beforeBlend=(await inspect(blend)).value;await focus(blend);await cdp.key('ArrowDown','ArrowDown',40);await cdp.key('Enter','Enter',13);
      assert.notEqual((await inspect(blend)).value,beforeBlend,label+': blend keyboard changes');
      const beforeMaster=(await inspect(selectors.master)).value;await focus(selectors.master);await cdp.key('ArrowLeft','ArrowLeft',37);
      assert.notEqual((await inspect(selectors.master)).value,beforeMaster,label+': master keyboard changes');
      // The deliberate confirmation is the in-app dialog (never a browser prompt):
      // no command may escape the guard, and nothing browser-native may open.
      const dlg='[data-in-app-dialog]',dlgCancel=dlg+' [data-in-app-dialog-cancel]',dlgConfirm=dlg+' [data-in-app-dialog-confirm]';
      const blackoutState=()=>cdp.evaluate('document.querySelector('+JSON.stringify(selectors.guardedBlackout)+').getAttribute("aria-pressed")');
      const dialogOpen=()=>cdp.evaluate('!!document.querySelector('+JSON.stringify(dlg+'[open]')+')');
      const dialogsBefore=cdp.dialogs.length;
      const blackoutBefore=await blackoutState();
      await focus(selectors.guardedBlackout);await cdp.key('Enter','Enter',13);
      await waitFor(dialogOpen,label+': in-app blackout dialog opens');
      const shown=await cdp.evaluate('(()=>{const d=document.querySelector('+JSON.stringify(dlg)+');const r=d.getBoundingClientRect();const title=document.getElementById(d.getAttribute("aria-labelledby"));const detail=document.getElementById(d.getAttribute("aria-describedby"));return {role:d.getAttribute("role"),modal:d.matches(":modal"),ariaModal:d.getAttribute("aria-modal"),tone:d.dataset.tone,title:title?.innerText,detail:detail?.innerText,focus:document.activeElement?.matches("[data-in-app-dialog-cancel]"),hazard:!!d.querySelector(".inAppDialogHazard"),inside:r.left>=0&&r.top>=0&&r.right<=innerWidth&&r.bottom<=innerHeight,buttons:[...d.querySelectorAll("button")].map(b=>({text:b.innerText.trim(),h:Math.round(b.getBoundingClientRect().height),w:Math.round(b.getBoundingClientRect().width)}))}})()');
      assert.equal(shown.role,'alertdialog',label+': dialog role');
      assert.ok(shown.modal&&shown.ariaModal==='true',label+': dialog is modal');
      assert.equal(shown.tone,'danger',label+': blackout is a danger dialog');
      assert.ok(shown.hazard,label+': danger dialog carries the hazard stripe');
      assert.ok(shown.inside,label+': dialog fits the viewport');
      assert.ok(shown.focus,label+': a danger dialog starts focused on Cancel');
      assert.ok(shown.buttons.length===2&&shown.buttons.every(b=>b.h>=44&&b.w>=44),label+': dialog targets are at least 44px '+JSON.stringify(shown.buttons));
      if(locale==='ja'){assert.match(shown.title,/ブラックアウト/,label+': Japanese blackout title');assert.match(shown.detail,/即座/,label+': Japanese consequence');}
      else{assert.match(shown.title,/Blackout video output\?/,label+': English blackout title');assert.match(shown.detail,/Program goes black immediately/,label+': English consequence');}
      await writeFile(join(evidenceDir,label+'-blackout-dialog.png'),Buffer.from((await cdp.capture({format:'png',captureBeyondViewport:false})).data,'base64'));
      // Perform and global shortcuts are suspended while the question is open.
      await cdp.key('?','Slash');
      assert.equal(await cdp.evaluate('!!document.querySelector("[data-video-perform-legend][open]")'),false,label+': Perform shortcuts are suspended under the dialog');
      // Focus is trapped: Tab never leaves the dialog.
      for(let i=0;i<5;i++){await cdp.key('Tab','Tab',9);assert.ok(await cdp.evaluate('document.activeElement?.closest("[data-in-app-dialog]")!==null'),label+': Tab stays inside the dialog');}
      await focus(dlgCancel);
      // Enter with focus on Cancel does nothing harmful and closes the question.
      await cdp.key('Enter','Enter',13);
      await waitFor(async()=>!(await dialogOpen()),label+': Enter on Cancel closes the dialog');
      assert.equal(await blackoutState(),blackoutBefore,label+': Enter on Cancel preserves blackout');
      assert.ok(await cdp.evaluate('document.activeElement?.matches('+JSON.stringify(selectors.guardedBlackout)+')'),label+': focus returns to the Blackout key');
      // Escape cancels too.
      await cdp.key('Enter','Enter',13);await waitFor(dialogOpen,label+': dialog reopens');
      await cdp.key('Escape','Escape',27);
      await waitFor(async()=>!(await dialogOpen()),label+': Escape cancels');
      assert.equal(await blackoutState(),blackoutBefore,label+': Escape preserves blackout');
      // Ctrl+Shift+B opens the same guarded dialog.
      await cdp.evaluate('document.activeElement?.blur()');
      await cdp.send('Input.dispatchKeyEvent',{type:'keyDown',key:'B',code:'KeyB',windowsVirtualKeyCode:66,modifiers:10});
      await cdp.send('Input.dispatchKeyEvent',{type:'keyUp',key:'B',code:'KeyB',windowsVirtualKeyCode:66,modifiers:10});
      await waitFor(dialogOpen,label+': Ctrl+Shift+B opens the same guarded dialog');
      assert.equal(await cdp.evaluate('document.querySelector('+JSON.stringify(dlg)+').dataset.tone'),'danger',label+': Ctrl+Shift+B dialog is the danger dialog');
      await focus(dlgCancel);await cdp.key('Enter','Enter',13);
      await waitFor(async()=>!(await dialogOpen()),label+': Ctrl+Shift+B dialog cancels');
      assert.equal(await blackoutState(),blackoutBefore,label+': Ctrl+Shift+B cancel preserves blackout');
      // New Project in Perform asks in the app as well (Ctrl+N); declining changes nothing.
      await cdp.evaluate('document.activeElement?.blur()');
      await cdp.send('Input.dispatchKeyEvent',{type:'keyDown',key:'n',code:'KeyN',windowsVirtualKeyCode:78,modifiers:2});
      await cdp.send('Input.dispatchKeyEvent',{type:'keyUp',key:'n',code:'KeyN',windowsVirtualKeyCode:78,modifiers:2});
      await waitFor(dialogOpen,label+': New Project in Perform opens the in-app dialog');
      const newProjectDialog=await cdp.evaluate('(()=>{const d=document.querySelector('+JSON.stringify(dlg)+');return {title:document.getElementById(d.getAttribute("aria-labelledby")).innerText,tone:d.dataset.tone,cancelFocused:document.activeElement?.matches("[data-in-app-dialog-cancel]")}})()');
      assert.match(newProjectDialog.title,locale==='ja'?/新規プロジェクト/:/new project/i,label+': New Project dialog wording');
      assert.ok(newProjectDialog.tone==='danger'&&newProjectDialog.cancelFocused,label+': New Project dialog starts on Cancel');
      await cdp.key('Escape','Escape',27);
      await waitFor(async()=>!(await dialogOpen()),label+': declining New Project closes the dialog');
      // Confirming proceeds to the existing blackout path (the browser preview has no desktop backend).
      await focus(selectors.guardedBlackout);await cdp.key('Enter','Enter',13);await waitFor(dialogOpen,label+': dialog opens for confirmation');
      await cdp.evaluate('document.querySelector('+JSON.stringify(dlgConfirm)+').click()');
      await waitFor(async()=>!(await dialogOpen()),label+': Confirm closes the dialog');
      assert.equal(cdp.dialogs.length,dialogsBefore,label+': no browser dialog ("tauri.localhost says") ever opened');
      // Legend is on demand and must never cause page scroll.
      await focus(selectors.legend);await cdp.key('Enter','Enter',13);
      await waitFor(()=>cdp.evaluate('!!document.querySelector("[data-video-perform-legend][open]")'),label+': keyboard legend shown');
      assert.equal((await inspect('[data-video-perform-legend]')).failure,null,label+': legend visible');
      for (const [key,code] of [['l','KeyL'],['p','KeyP'],['1','Digit1']]) {
        await cdp.key(key,code);
        assert.ok(await cdp.evaluate('!!document.querySelector("[data-video-perform-layout] [data-video-perform-legend][open]")'),label+': '+key+' leaves modal and Perform workspace intact');
      }
      if(locale==='ja')assert.doesNotMatch(await cdp.evaluate('document.querySelector("[data-video-perform-legend]").innerText'),/\?{2,}|\uFFFD/,label+': Japanese legend has no replacement runs');
      await writeFile(join(evidenceDir,label+'-legend.png'),Buffer.from((await cdp.capture({format:'png',captureBeyondViewport:false})).data,'base64'));
      await cdp.key('Escape','Escape',27);
      await waitFor(()=>cdp.evaluate('!document.querySelector("[data-video-perform-legend][open]")'),label+': keyboard legend hidden');
      for(const selector of required.filter(s=>s!==selectors.launch))assert.equal((await inspect(selector)).failure,null,label+': after interactions '+selector);
      assert.deepEqual(await cdp.evaluate('[scrollX,scrollY,document.documentElement.scrollWidth,document.documentElement.scrollHeight]'),[0,0,width,height],label+': no outer scroll after interaction');
      await writeFile(join(evidenceDir,label+'-after.png'),Buffer.from((await cdp.capture({format:'png',captureBeyondViewport:false,optimizeForSpeed:true},30_000)).data,'base64'));
      assert.deepEqual(cdp.errors,[],label+': console errors/warnings/runtime exceptions');
      results.push({label,identity,geometry,visibleColumns:visibleColumns.length,fullLayerRows:matrixDensity.fullRows,totalLayerRows:matrixDensity.totalRows,fxTab,monitors,panes,dragged,tabTargets:reached.size,interactions:'matrix arrows/Home/End; numeric queue native-required refusal; Take; opacity/blend/master keyboard; blackout first-activation guard; legend',consoleErrors:cdp.errors});
      console.log('PASS '+label+' ('+visibleColumns.length+' columns, '+matrixDensity.fullRows+'/'+matrixDensity.totalRows+' full layer rows)');
    }
  }
  assert.equal(results.length, process.env.SYNDOCAL_PERFORM_FOCUS_ONLY === "1" ? 0 : 6, "all requested viewport/locale cases actually ran");
  const focusResults = [];
  for (const performance of [true, false]) {
    currentCase = `shared-bank-focus-${performance ? "perform" : "existing-control"}`;
    await freshTarget();
    await cdp.send("Page.navigate", { url: `${origin}__perform-focus` });
    await waitFor(() => cdp.evaluate("!!window.__performFocus && !!document.querySelector('[data-video-clip-slot-pad-index]')"), `${currentCase}: fixture ready`);
    await cdp.evaluate(`window.__performFocus.setPerformance(${performance}); window.__focusOriginal = document.querySelector('[data-video-clip-slot-pad-index="0"]'); window.__focusOriginal.focus(); true`);
    const focusState = () => cdp.evaluate(`({ connected: window.__focusOriginal.isConnected,
      sameElement: document.querySelector('[data-video-clip-slot-pad-index="0"]') === window.__focusOriginal,
      focused: document.activeElement === window.__focusOriginal,
      active: window.__focusOriginal.closest('article').classList.contains('active'),
      queued: window.__focusOriginal.closest('article').classList.contains('queued'),
      badge: window.__focusOriginal.querySelector('em')?.textContent ?? null })`);
    assert.deepEqual(await focusState(), { connected: true, sameElement: true, focused: true, active: true, queued: false, badge: "LIVE" }, `${currentCase}: initial focused active pad`);
    await cdp.evaluate("window.__performFocus.refresh(); true");
    assert.deepEqual(await focusState(), { connected: true, sameElement: true, focused: true, active: true, queued: false, badge: "LIVE" }, `${currentCase}: runtime playhead replacement preserves focus`);
    await cdp.evaluate("window.__performFocus.queue(); true");
    assert.deepEqual(await focusState(), { connected: true, sameElement: true, focused: true, active: false, queued: true, badge: "QUEUED" }, `${currentCase}: active/queued badges stay reactive without losing focus`);
    await cdp.evaluate("window.__performFocus.replaceLayer(); true");
    const finalState = await focusState();
    assert.deepEqual(finalState, { connected: true, sameElement: true, focused: true, active: false, queued: true, badge: "QUEUED" }, `${currentCase}: same-ID layer and authored slot replacements preserve focus`);
    assert.ok(await cdp.evaluate("document.body.innerText.includes('Updated layer')"), `${currentCase}: replaced layer remains reactive`);
    await cdp.key("ArrowRight", "ArrowRight", 39);
    assert.equal(await cdp.evaluate("document.activeElement?.dataset.videoClipSlotPadIndex"), "1", `${currentCase}: arrows work after refresh`);
    await cdp.key("Enter", "Enter", 13);
    assert.deepEqual(await cdp.evaluate("window.__performFocus.queueCalls"), [102], `${currentCase}: Enter queues exact focused slot after refresh`);
    assert.deepEqual(cdp.errors, [], `${currentCase}: browser errors/warnings`);
    focusResults.push({ label: currentCase, finalState, queuedSlotId: 102, consoleErrors: cdp.errors });
    console.log(`PASS ${currentCase}`);
  }

  currentCase='matrix-runtime-focus';
  await freshTarget();await cdp.send('Emulation.setDeviceMetricsOverride',{width:1366,height:768,deviceScaleFactor:1,mobile:false});
  await cdp.send('Page.navigate',{url:origin+'__perform-matrix-focus'});
  await waitFor(()=>cdp.evaluate('!!window.__matrixFocus && !!document.querySelector("[data-video-perform-cell-index]")'),currentCase+': fixture ready');
  const matrixFocusedSelector='[data-video-perform-row="1"] [data-video-perform-cell-index="0"]';
  await cdp.evaluate('window.__matrixOriginal=document.querySelector('+JSON.stringify(matrixFocusedSelector)+');window.__matrixOriginal.focus();true');
  const matrixState=()=>cdp.evaluate('({connected:window.__matrixOriginal.isConnected,sameElement:document.querySelector('+JSON.stringify(matrixFocusedSelector)+')===window.__matrixOriginal,focused:document.activeElement===window.__matrixOriginal,state:window.__matrixOriginal.closest("[data-video-perform-cell-state]").dataset.videoPerformCellState})');
  assert.deepEqual(await matrixState(),{connected:true,sameElement:true,focused:true,state:'LIVE'},currentCase+': initial live');
  const matrixCellState = (slotId) => cdp.evaluate('(()=>{const cell=document.querySelector("[data-video-clip-slot-id=\\"'+slotId+'\\"]");return {state:cell.dataset.videoPerformCellState,preview:cell.classList.contains("preview"),text:cell.querySelector(".videoPerformClipState").textContent}})()');
  // Per-cell Inspect/More moved: right-click selects the clip and opens options; the corner buttons act on the selection.
  assert.deepEqual(await cdp.evaluate('[document.querySelectorAll(".videoPerformClipActions").length,document.querySelectorAll(".videoPerformClipNumber").length,document.querySelectorAll(".videoPerformMatrixCell button").length===document.querySelectorAll(".videoPerformMatrixCell").length]'),[0,0,true],currentCase+': cells hold only the trigger button');
  assert.equal(await cdp.evaluate('document.querySelector(".videoPerformMatrixHead").getBoundingClientRect().height'),44,currentCase+': corner head is 44px');
  const contextmenu=(slotSelector)=>cdp.evaluate('(()=>{const pad=document.querySelector('+JSON.stringify(slotSelector)+');return !pad.dispatchEvent(new MouseEvent("contextmenu",{bubbles:true,cancelable:true}))})()');
  assert.equal(await contextmenu('[data-video-clip-slot-id="102"] [data-video-perform-cell-index]'),true,currentCase+': contextmenu is consumed by the clip trigger');
  assert.deepEqual(await cdp.evaluate('window.__matrixFocus.moreCalls'),[{layer:1,slot:102,selected:102,anchor:'cell'}],currentCase+': contextmenu selects layer+slot then opens clip options anchored to the pad');
  assert.deepEqual(await cdp.evaluate('window.__matrixFocus.queueCalls'),[],currentCase+': contextmenu never queues a clip');
  assert.equal(await cdp.evaluate('document.querySelector("[data-video-perform-inspect]").disabled'),false,currentCase+': corner Inspect enabled with a selected clip');
  await cdp.evaluate('document.querySelector("[data-video-perform-inspect]").click();true');
  assert.deepEqual(await cdp.evaluate('window.__matrixFocus.inspectCalls'),[102],currentCase+': corner Inspect uses existing thumbnail route for the selected clip');
  await cdp.evaluate('document.querySelector("[data-video-perform-clip-options]").click();true');
  assert.deepEqual(await cdp.evaluate('window.__matrixFocus.moreCalls.at(-1)'),{layer:1,slot:102,selected:102,anchor:'corner'},currentCase+': corner Clip… opens options for the selected clip anchored to itself');
  assert.equal(await contextmenu('[data-video-clip-slot-index="5"][data-video-clip-slot-layer-id="1"] [data-video-perform-cell-index]'),true,currentCase+': contextmenu on an empty pad is consumed');
  assert.deepEqual(await cdp.evaluate('window.__matrixFocus.moreCalls.at(-1)'),{layer:1,slot:null,selected:null,anchor:'cell'},currentCase+': contextmenu on an empty pad selects no slot');
  assert.equal(await cdp.evaluate('document.querySelector("[data-video-perform-inspect]").disabled'),true,currentCase+': corner Inspect disabled without a selected clip');
  await cdp.evaluate('document.querySelector("[data-video-perform-inspect]").click();true');
  assert.deepEqual(await cdp.evaluate('window.__matrixFocus.inspectCalls'),[102],currentCase+': disabled Inspect does not route');
  await contextmenu('[data-video-clip-slot-id="102"] [data-video-perform-cell-index]');
  assert.deepEqual(await matrixCellState(102),{state:'Loaded',preview:false,text:'Loaded'},currentCase+': thumbnail inspection does not invent a previewing/playback state');
  await cdp.evaluate('window.__matrixFocus.setPreviewLayerId(1);true');
  const livePreview=await matrixCellState(101);
  assert.equal(livePreview.state,'LIVE',currentCase+': Program playback remains authoritative');
  assert.equal(livePreview.preview,true,currentCase+': current active clip exposes real layer Preview');
  assert.match(livePreview.text,/Preview/,currentCase+': live clip Preview is also indicated by text');
  assert.equal((await matrixCellState(102)).preview,false,currentCase+': inactive inspected clip is never marked Preview');
  await cdp.evaluate('window.__matrixFocus.setRuntime({layer_id:1,active_slot_id:101,queued_slot_id:null,pending_launch:null,transition:null,playhead_ms:0,playing:false});true');
  assert.equal((await matrixCellState(101)).state,'Preview',currentCase+': active source staged in Preview has a truthful Preview state');
  await cdp.evaluate('window.__matrixFocus.setPreviewLayerId(null);window.__matrixFocus.setRuntime({layer_id:1,active_slot_id:101,queued_slot_id:null,pending_launch:null,transition:null,playhead_ms:0,playing:true});window.__matrixOriginal.focus();true');
  await cdp.evaluate('window.__matrixFocus.refresh();true');
  assert.deepEqual(await matrixState(),{connected:true,sameElement:true,focused:true,state:'LIVE'},currentCase+': polling replacement retains focused element');
  await cdp.evaluate('window.__matrixFocus.queue();true');
  assert.deepEqual(await matrixState(),{connected:true,sameElement:true,focused:true,state:'Queued'},currentCase+': queued state reactive');
  await cdp.evaluate('window.__matrixFocus.replaceLayer();true');
  assert.deepEqual(await matrixState(),{connected:true,sameElement:true,focused:true,state:'Queued'},currentCase+': same-ID authored layer replacement retains focus');
  assert.ok(await cdp.evaluate('document.body.innerText.includes("Updated matrix layer")'),currentCase+': layer label reactive');
  await cdp.key('ArrowRight','ArrowRight',39);
  assert.equal(await cdp.evaluate('document.activeElement?.dataset.videoPerformCellIndex'),'1',currentCase+': right after refresh');
  await cdp.key('2','Digit2',50);
  assert.deepEqual(await cdp.evaluate('window.__matrixFocus.queueCalls'),[[1,102]],currentCase+': numeric key queues exact slot');
  await cdp.key('ArrowDown','ArrowDown',40);await cdp.key('1','Digit1',49);
  assert.deepEqual(await cdp.evaluate('window.__matrixFocus.queueCalls'),[[1,102],[2,201]],currentCase+': selected row changes before exact queue');
  await cdp.key('Enter','Enter',13);await cdp.key(' ','Space',32);
  assert.equal(await cdp.evaluate('window.__matrixFocus.takes.length'),2,currentCase+': Enter and Space take once');
  // Editing faders must retain native arrow-key behavior and not launch clips.
  await cdp.evaluate('document.querySelector('+JSON.stringify('[data-video-layer-opacity="2"]')+').focus()');
  await cdp.key('1','Digit1',49);await cdp.key('ArrowRight','ArrowRight',39);
  assert.equal(await cdp.evaluate('window.__matrixFocus.queueCalls.length'),2,currentCase+': editable controls do not trigger clips');
  await cdp.evaluate("document.querySelector('[data-fx-catalog-card]').focus()");
  await cdp.key('Enter','Enter',13);
  assert.deepEqual(await cdp.evaluate('window.__matrixFocus.effectAdds'),[['builtin:invert',2]],currentCase+': catalog Enter adds the exact effect to layer 2');
  assert.equal(await cdp.evaluate('window.__matrixFocus.takes.length'),2,currentCase+': catalog Enter never Takes a queued clip');
  // Dragging a card onto a layer row adds to that exact row after a visible LIVE confirmation; declining adds nothing.
  const dropOnRow = (layer, payload) => cdp.evaluate(`(()=>{const row=document.querySelector('[data-video-perform-row="${layer}"]');const dt=new DataTransfer();dt.setData('application/x-syndocal-fx',${JSON.stringify(payload)});const over=new DragEvent('dragover',{bubbles:true,cancelable:true,dataTransfer:dt});row.dispatchEvent(over);row.dispatchEvent(new DragEvent('drop',{bubbles:true,cancelable:true,dataTransfer:dt}));return over.defaultPrevented})()`);
  cdp.dialogs.length = 0;
  const inApp = '[data-in-app-dialog]';
  assert.equal(await dropOnRow(3, 'builtin:mirror'), true, currentCase+': a layer row accepts effect drops');
  await waitFor(() => cdp.evaluate(`!!document.querySelector('${inApp}[open]')`), currentCase+': a row drop asks for confirmation in the app');
  const dropDialog = await cdp.evaluate(`(()=>{const d=document.querySelector('${inApp}');return {title:document.getElementById(d.getAttribute('aria-labelledby')).innerText,detail:document.getElementById(d.getAttribute('aria-describedby')).innerText,tone:d.dataset.tone}})()`);
  assert.match(dropDialog.title, /Layer 3/, currentCase+': confirmation names the destination layer');
  assert.match(dropDialog.detail, /LIVE/, currentCase+': confirmation states the LIVE consequence');
  assert.equal(cdp.dialogs.length, 0, currentCase+': the confirmation is never a browser dialog');
  await cdp.evaluate(`document.querySelector('${inApp} [data-in-app-dialog-cancel]').click()`);
  await waitFor(() => cdp.evaluate(`!document.querySelector('${inApp}[open]')`), currentCase+': declining closes the dialog');
  assert.equal((await cdp.evaluate('window.__matrixFocus.effectAdds')).length, 1, currentCase+': declined drop adds nothing');
  await dropOnRow(3, 'builtin:mirror');
  await waitFor(() => cdp.evaluate(`!!document.querySelector('${inApp}[open]')`), currentCase+': the second drop asks again');
  await cdp.evaluate(`document.querySelector('${inApp} [data-in-app-dialog-confirm]').click()`);
  await waitFor(() => cdp.evaluate(`!document.querySelector('${inApp}[open]')`), currentCase+': accepting closes the dialog');
  assert.deepEqual(await cdp.evaluate('window.__matrixFocus.effectAdds'), [['builtin:invert',2],['builtin:mirror',3]], currentCase+': accepted drop adds the exact card to the dropped row');
  await dropOnRow(3, 'builtin:unknown-or-untrusted-source');
  assert.equal((await cdp.evaluate('window.__matrixFocus.effectAdds')).length, 2, currentCase+': an unknown drag payload adds nothing');
  await cdp.evaluate("window.__catalogFocus=document.querySelector('[data-fx-catalog-search]');window.__catalogFocus.focus();window.__matrixFocus.replaceLayer();true");
  assert.equal(await cdp.evaluate("document.activeElement===window.__catalogFocus && window.__catalogFocus.isConnected"),true,currentCase+': browser search focus survives layer polling');
  assert.deepEqual(cdp.errors,[],currentCase+': console errors/warnings');
  focusResults.push({label:currentCase,focusRetainedAcross:'playhead,queue,same-ID layer/slots',queueCalls:await cdp.evaluate('window.__matrixFocus.queueCalls'),takes:2,consoleErrors:cdp.errors});
  console.log('PASS '+currentCase);

  currentCase='quick-fx-runtime-focus';
  await freshTarget();
  await cdp.send('Page.navigate',{url:origin+'__perform-quick-fx-focus'});
  await waitFor(()=>cdp.evaluate('!!window.__quickFxFocus && !!document.querySelector("[data-perform-fx-control] input")'),currentCase+': fixture ready');
  const fxParameter='[data-perform-fx-detail="0"] [data-perform-fx-control="gain"] input';
  const fxBypass='[data-perform-fx-stage="0"] [data-perform-fx-bypass]';
  const rememberFx = (selector) => cdp.evaluate('window.__fxOriginal=document.querySelector('+JSON.stringify(selector)+');window.__fxOriginal.focus();true');
  const fxState = (selector) => cdp.evaluate('({connected:window.__fxOriginal.isConnected,sameElement:document.querySelector('+JSON.stringify(selector)+')===window.__fxOriginal,focused:document.activeElement===window.__fxOriginal,value:window.__fxOriginal.value,pressed:window.__fxOriginal.getAttribute("aria-pressed")})');
  await rememberFx(fxParameter);
  assert.deepEqual(await fxState(fxParameter),{connected:true,sameElement:true,focused:true,value:'0.5',pressed:null},currentCase+': initial parameter');
  await cdp.evaluate('window.__quickFxFocus.refresh();true');
  assert.deepEqual(await fxState(fxParameter),{connected:true,sameElement:true,focused:true,value:'0.5',pressed:null},currentCase+': replacement polling snapshot retains focused parameter');
  await cdp.evaluate('window.__quickFxFocus.replace(0.65,false);true');
  assert.deepEqual(await fxState(fxParameter),{connected:true,sameElement:true,focused:true,value:'0.65',pressed:null},currentCase+': same-ID authored layer/stage/control replacement retains focused parameter and updates value');
  assert.ok(await cdp.evaluate('document.body.innerText.includes("Updated FX layer")'),currentCase+': layer replacement remains reactive');
  // Parameter publication is a completed gesture, avoiding dropped trailing
  // samples and one project transaction per pointer movement.
  await cdp.evaluate('window.__fxOriginal.value="0.8";window.__fxOriginal.dispatchEvent(new Event("change",{bubbles:true}));true');
  assert.deepEqual(await cdp.evaluate('window.__quickFxFocus.calls.at(-1)'),['control',1,0,'gain',[0.8,0,0,0]],currentCase+': refreshed parameter routes exact scoped command');
  assert.equal((await fxState(fxParameter)).focused,true,currentCase+': focused parameter survives its own publication');
  await rememberFx(fxBypass);
  await cdp.evaluate('window.__quickFxFocus.refresh();window.__quickFxFocus.replace(0.75,true);true');
  assert.deepEqual(await fxState(fxBypass),{connected:true,sameElement:true,focused:true,value:'',pressed:'true'},currentCase+': focused stage button survives polling and authored replacement with reactive enabled state');
  await cdp.key('Enter','Enter',13);
  assert.deepEqual(await cdp.evaluate('window.__quickFxFocus.calls.at(-1)'),['enabled',1,0,false],currentCase+': Enter bypasses exact scoped stage after refresh');
  assert.deepEqual(await fxState(fxBypass),{connected:true,sameElement:true,focused:true,value:'',pressed:'false'},currentCase+': scoped bypass publication retains focus');
  assert.deepEqual(cdp.errors,[],currentCase+': console errors/warnings');
  focusResults.push({label:currentCase,focusRetainedAcross:'polling and same-ID authored layer/stage/control replacement',scopedCalls:await cdp.evaluate('window.__quickFxFocus.calls'),consoleErrors:cdp.errors});
  console.log('PASS '+currentCase);
  currentCase='divider-persistence';
  await freshTarget();
  await cdp.send('Emulation.setDeviceMetricsOverride',{width:1366,height:768,deviceScaleFactor:1,mobile:false});
  const persistedScript=await cdp.send('Page.addScriptToEvaluateOnNewDocument',{source:"localStorage.setItem('syndocal.uiLocale.v1','en');localStorage.setItem('syndocal.workspaceLayout.v1',JSON.stringify({workspace_tab:'touch',control_mode:'perform',touch_control_domain:'video',perform_top_split_ratio:0.66,perform_lower_split_ratio:0.6}));"});
  await cdp.send('Page.navigate',{url:`${origin}?syndocalViewportFixture=vj-bank&performCase=persistence`});
  await waitFor(()=>cdp.evaluate("!!document.querySelector('[data-workspace-option=touch]')"),currentCase+': workspace chrome');
  await cdp.evaluate("document.querySelector('[data-workspace-option=touch]').click(); document.querySelector('[data-control-domain=video]').click()");
  await waitFor(()=>cdp.evaluate('!!document.querySelector('+JSON.stringify(root)+') && !!document.querySelector('+JSON.stringify(selectors.launch)+')'),currentCase+': Perform loaded');
  await cdp.send('Page.removeScriptToEvaluateOnNewDocument',{identifier:persistedScript.identifier});
  const restoredRatios=await cdp.evaluate('(()=>{const a=(k)=>Number(document.querySelector("[data-video-perform-layout] [data-workspace-splitter=\\""+k+"\\"]").getAttribute("aria-valuenow"));return {top:a("upper-lower"),lower:a("lower-left-right")}})()');
  assert.deepEqual(restoredRatios,{top:66,lower:60},currentCase+': a stored Perform layout restores both divider positions');
  assert.deepEqual(cdp.errors,[],currentCase+': console errors/warnings');
  focusResults.push({label:currentCase,restoredRatios,consoleErrors:cdp.errors});
  console.log('PASS '+currentCase);
  assert.deepEqual(cdp.errors, [], "browser console errors/warnings and runtime exceptions");
  await writeFile(join(evidenceDir, "results.json"), JSON.stringify({ browser: "raw CDP (Browser plugin not available; no Playwright dependency)", results, focusResults, consoleErrors: cdp.errors }, null, 2));
  console.log(`Perform layout: ${results.length} viewport/locale cases and ${focusResults.length} runtime-focus cases passed; evidence ${evidenceDir}`);
} catch (error) {
  if (cdp?.socket?.readyState === WebSocket.OPEN) {
    const diagnostic = await cdp.evaluate("({url:location.href,text:document.body.innerText,workspace:localStorage.getItem('syndocal.workspaceLayout.v1'),keys:window.__performKeys})").catch(() => null);
    await writeFile(join(evidenceDir, "failure.json"), JSON.stringify({ currentCase, error: error.stack, diagnostic, consoleErrors: cdp.errors, results }, null, 2));
    const screenshot = await cdp.capture( { format: "png" }).catch(() => null);
    if (screenshot) await writeFile(join(evidenceDir, `${currentCase}-failure.png`), Buffer.from(screenshot.data, "base64"));
  }
  throw error;
} finally {
  if (cdp?.socket?.readyState === WebSocket.OPEN) {
    await cdp.send("Browser.close").catch(() => {});
    cdp.socket.close();
  }
  if (browser && browser.exitCode === null) {
    try { await waitFor(() => browser.exitCode !== null || browser.signalCode !== null, "owned browser exit", 5_000); }
    catch { browser.kill(); }
  }
  if (server) await server.close();
  if (profile) {
    // Verify the final absolute target before recursively removing our profile.
    const profileTarget = resolve(profile);
    assert.equal(dirname(profileTarget), resolve(tmpdir()), "profile cleanup must stay in the named temporary directory");
    assert.ok(basename(profileTarget).startsWith("syndocal-perform-check-"), "profile cleanup must use the owned prefix");
    await rm(profileTarget, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
  }
}
