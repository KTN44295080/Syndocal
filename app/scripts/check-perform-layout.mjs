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
  tap: `${root} [data-video-perform-tap]`,
  fxEditor: `${root} [data-video-perform-fx-editor]`,
  guardedBlackout: `${root} [data-video-perform-blackout]`,
  legend: `${root} [data-video-perform-legend-toggle]`,
  blackout: "[data-global-operator-action='video-blackout']",
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
 import { viewportFixtureData } from '/src/viewportFixtureData.ts';
 import '/src/styles.css';
 const initial=viewportFixtureData.vjBankLayers.slice(0,3).map((layer,i)=>({...structuredClone(layer),clip_slots:[101,102].map(id=>({id:i*100+id,media_asset_id:id,launch_quantization:'Immediate'}))}));
 const [layers,setLayers]=createSignal(initial);
 const [selectedLayer,setSelectedLayer]=createSignal(1),[selected,setSelected]=createSignal(101);
 const [runtime,setRuntime]=createSignal({layer_id:1,active_slot_id:101,queued_slot_id:null,pending_launch:null,transition:null,playhead_ms:0,playing:true});
 const queueCalls=[],takes=[];
 Object.assign(window,{__matrixFocus:{
 refresh:()=>setRuntime({...runtime(),playhead_ms:runtime().playhead_ms+250}),
 queue:()=>setRuntime({...runtime(),active_slot_id:null,queued_slot_id:101}),
 replaceLayer:()=>setLayers(layers().map(l=>({...l,label:l.id===1?'Updated matrix layer':l.label,clip_slots:l.clip_slots.map(s=>({...s}))}))),
 queueCalls,takes,
 }});
 const bank={mode:'control',get layers(){return layers()},get layer(){return layers().find(l=>l.id===selectedLayer())},get runtime(){return runtime()},assets:[101,102].map(id=>({id,label:'Focus asset '+id,source:initial[0].source})),thumbnails:{},get selectedSlotId(){return selected()},onSelectLayer:setSelectedLayer,onSelect:setSelected,onQueue:id=>queueCalls.push([selectedLayer(),id]),onPreview:()=>{},onMore:()=>{},onImport:()=>{},onCancelQueue:()=>{},onSetQuantization:()=>{},transitionKind:'Cut',transitionDurationUnit:'Milliseconds',transitionDurationMilliunits:750,onSetTransitionKind:()=>{},onSetTransitionDurationUnit:()=>{},onSetTransitionDurationMilliunits:()=>{}};
 const list={get layers(){return layers()},onSetLayerState:(id,state)=>setLayers(layers().map(l=>l.id===id?{...l,state}:l)),onSetBlendMode:(id,blend_mode)=>setLayers(layers().map(l=>l.id===id?{...l,blend_mode}:l))};
 render(()=><div style={{height:'680px',width:'1300px'}}><VideoPerformMatrix bank={bank} layerList={list} runtimes={[runtime()]} availabilityById={{}} previewLayerId={null} thumbnailsAuthorized={false} onStop={()=>{}} keyboardEnabled={true} onTake={()=>takes.push(true)}/></div>,document.getElementById('root'));
`;

const focusFixturePlugin = {
  name: "perform-focus-regression-fixture",
  enforce: "pre",
  resolveId(id) { if (id === focusFixturePath) return focusFixtureId; if(id===matrixFocusPath)return matrixFocusId; },
  load(id) { if (id === focusFixtureId) return focusFixtureSource; if(id===matrixFocusId)return matrixFocusSource; },
  configureServer(instance) {
    instance.middlewares.use((request, response, next) => {
      if (request.url !== "/__perform-focus" && request.url !== "/__perform-matrix-focus") return next();
      response.setHeader("Content-Type", "text/html");
      response.end('<!doctype html><title>Shared bank focus regression</title><div id="root"></div><script type="module" src="' + (request.url === '/__perform-matrix-focus' ? matrixFocusPath : focusFixturePath) + '"></script>');
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
  server = await createServer({ root: appRoot, server: { host: "127.0.0.1", port: vitePort, strictPort: true }, clearScreen: false, plugins: [focusFixturePlugin] });
  await server.listen();
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
      await cdp.send("Page.addScriptToEvaluateOnNewDocument", { source: `localStorage.setItem('syndocal.uiLocale.v1', ${JSON.stringify(locale)}); localStorage.setItem('syndocal.workspaceLayout.v1', JSON.stringify({workspace_tab:'control',control_mode:'perform'}));` }).then(async ({ identifier }) => {
        await cdp.send("Page.navigate", { url });
        await waitFor(() => cdp.evaluate(`location.href === ${JSON.stringify(url)} && document.readyState === 'complete'`), `${label} navigation committed`);
        await cdp.send("Page.bringToFront");
        await waitFor(() => cdp.evaluate("!!document.querySelector('#edit-domain-tab-perform')"), `${label} workspace chrome`);
        // Existing vj-bank fixture intentionally opens its historic Mixer desk.
        // Enter the new surface through the actual workspace control.
        await cdp.evaluate("document.querySelector('#edit-domain-tab-perform').click()");
        await waitFor(() => cdp.evaluate(`!!document.querySelector(${JSON.stringify(root)}) && !!document.querySelector(${JSON.stringify(selectors.launch)})`), `${label} loaded`);
        await cdp.send("Page.removeScriptToEvaluateOnNewDocument", { identifier });
      });
      await cdp.evaluate("document.fonts.ready.then(() => true)");
      const expectedHeading = locale === "ja" ? "ライブ映像" : "Perform";
      await waitFor(() => cdp.evaluate(`document.querySelector('${root} .videoPerformDeskHeader h2')?.textContent === ${JSON.stringify(expectedHeading)}`), `${label}: rendered locale`);
      await cdp.evaluate("window.__performKeys=[]; window.__performClicks=[]; document.addEventListener('keydown', e => queueMicrotask(() => window.__performKeys.push({key:e.key,prevented:e.defaultPrevented,target:e.target.outerHTML.slice(0,240)}))); document.addEventListener('click', e => window.__performClicks.push(e.target.closest('[data-global-operator-action]')?.dataset.globalOperatorAction ?? 'other'))");
      const identity = await cdp.evaluate(`({url:location.href,title:document.title,text:document.body.innerText.length,overlay:!!document.querySelector('vite-error-overlay'),width:innerWidth,height:innerHeight,scrollWidth:document.documentElement.scrollWidth,scrollHeight:document.documentElement.scrollHeight})`);
      assert.equal(identity.url, url, label);
      assert.ok(identity.title && identity.text > 100, `${label}: meaningful application`);
      assert.equal(identity.overlay, false, `${label}: framework overlay`);
      assert.equal(identity.scrollWidth, width, `${label}: outer horizontal scroll`);
      assert.equal(identity.scrollHeight, height, `${label}: outer vertical scroll`);
      await writeFile(join(evidenceDir, `${label}-before.png`), Buffer.from((await cdp.send("Page.captureScreenshot", { format: "png", captureBeyondViewport: false, optimizeForSpeed: true }, 30_000)).data, "base64"));
      const inspect = (selector) => cdp.evaluate('('+inspectInPage+')('+JSON.stringify(selector)+')');
      const regions = ['matrix','transition','tempo','quick-fx','health'].map(name => root+' [data-video-perform-region="'+name+'"]');
      const required = [...Object.values(selectors), ...regions];
      const geometry = [];
      for (const selector of required) {
        const measurement = await inspect(selector); geometry.push(measurement);
        assert.equal(measurement.failure, null, label+': '+JSON.stringify(measurement));
        if (selector.startsWith(root) && !regions.includes(selector) && ![selectors.preview,selectors.program].includes(selector)) {
          assert.ok(measurement.height >= 44 && measurement.width >= 44, label+': preserved 44px control target '+selector);
        }
      }
      const rowSelector=root+' [data-video-perform-row="1"]';
      const visibleColumns=await cdp.evaluate('Array.from(document.querySelectorAll('+JSON.stringify(rowSelector+' [data-video-perform-cell-index]')+')).filter(e=>('+inspectInPage+')(e).failure===null).map(e=>Number(e.dataset.videoPerformCellIndex))');
      assert.ok(visibleColumns.length >= (width>=1366?8:7), label+': fully visible clip columns '+visibleColumns.length);
      if(width===1920)assert.ok(visibleColumns.length>8,label+': wide display exposes more than 8 columns');
      assert.ok(visibleColumns.every((x,i)=>x===i),label+': first clip columns visible');
      for(const attr of ['opacity','blend']){
        const selector=rowSelector+' [data-video-layer-'+attr+']';const measurement=await inspect(selector);geometry.push(measurement);
        assert.equal(measurement.failure,null,label+': strip '+attr);assert.ok(measurement.height>=44&&measurement.width>=44,label+': strip hit target '+attr);
      }
      const focus = (selector) => cdp.evaluate('document.querySelector('+JSON.stringify(selector)+').focus({preventScroll:true})');
      // Real Tab traversal catches hidden tab stops, focus traps and omitted controls.
      await cdp.evaluate('document.activeElement?.blur()');
      const tabCandidates=[root+' [data-video-perform-cell-index][tabindex="0"]',selectors.master,selectors.take,selectors.tap,selectors.guardedBlackout,selectors.legend,rowSelector+' [data-video-layer-opacity]',rowSelector+' [data-video-layer-blend]'];
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
      // Reject the deliberate confirmation: no command may escape the guard.
      const blackoutBefore=await cdp.evaluate('document.querySelector('+JSON.stringify(selectors.guardedBlackout)+').getAttribute("aria-pressed")');
      cdp.acceptDialogs=false;const dialogsBefore=cdp.dialogs.length;
      await focus(selectors.guardedBlackout);await cdp.key('Enter','Enter',13);
      await waitFor(()=>cdp.dialogs.length===dialogsBefore+1,label+': blackout confirmation guard');
      assert.equal(cdp.dialogs.at(-1).type,'confirm',label+': deliberate confirmation required');
      assert.equal(await cdp.evaluate('document.querySelector('+JSON.stringify(selectors.guardedBlackout)+').getAttribute("aria-pressed")'),blackoutBefore,label+': rejected guard preserves blackout');
      cdp.acceptDialogs=true;
      // Legend is on demand and must never cause page scroll.
      await focus(selectors.legend);await cdp.key('Enter','Enter',13);
      await waitFor(()=>cdp.evaluate('!!document.querySelector("[data-video-perform-legend][open]")'),label+': keyboard legend shown');
      assert.equal((await inspect('[data-video-perform-legend]')).failure,null,label+': legend visible');
      await writeFile(join(evidenceDir,label+'-legend.png'),Buffer.from((await cdp.send('Page.captureScreenshot',{format:'png',captureBeyondViewport:false})).data,'base64'));
      await cdp.key('Escape','Escape',27);
      await waitFor(()=>cdp.evaluate('!document.querySelector("[data-video-perform-legend][open]")'),label+': keyboard legend hidden');
      for(const selector of required.filter(s=>s!==selectors.launch))assert.equal((await inspect(selector)).failure,null,label+': after interactions '+selector);
      assert.deepEqual(await cdp.evaluate('[scrollX,scrollY,document.documentElement.scrollWidth,document.documentElement.scrollHeight]'),[0,0,width,height],label+': no outer scroll after interaction');
      await writeFile(join(evidenceDir,label+'-after.png'),Buffer.from((await cdp.send('Page.captureScreenshot',{format:'png',captureBeyondViewport:false,optimizeForSpeed:true},30_000)).data,'base64'));
      assert.deepEqual(cdp.errors,[],label+': console errors/warnings/runtime exceptions');
      results.push({label,identity,geometry,visibleColumns:visibleColumns.length,tabTargets:reached.size,interactions:'matrix arrows/Home/End; numeric queue native-required refusal; Take; opacity/blend/master keyboard; blackout first-activation guard; legend',consoleErrors:cdp.errors});
      console.log('PASS '+label+' ('+visibleColumns.length+' columns)');
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
  assert.deepEqual(cdp.errors,[],currentCase+': console errors/warnings');
  focusResults.push({label:currentCase,focusRetainedAcross:'playhead,queue,same-ID layer/slots',queueCalls:await cdp.evaluate('window.__matrixFocus.queueCalls'),takes:2,consoleErrors:cdp.errors});
  console.log('PASS '+currentCase);
  assert.deepEqual(cdp.errors, [], "browser console errors/warnings and runtime exceptions");
  await writeFile(join(evidenceDir, "results.json"), JSON.stringify({ browser: "raw CDP (Browser plugin not available; no Playwright dependency)", results, focusResults, consoleErrors: cdp.errors }, null, 2));
  console.log(`Perform layout: ${results.length} viewport/locale cases and ${focusResults.length} runtime-focus cases passed; evidence ${evidenceDir}`);
} catch (error) {
  if (cdp?.socket?.readyState === WebSocket.OPEN) {
    const diagnostic = await cdp.evaluate("({url:location.href,text:document.body.innerText,workspace:localStorage.getItem('syndocal.workspaceLayout.v1'),keys:window.__performKeys})").catch(() => null);
    await writeFile(join(evidenceDir, "failure.json"), JSON.stringify({ currentCase, error: error.stack, diagnostic, consoleErrors: cdp.errors, results }, null, 2));
    const screenshot = await cdp.send("Page.captureScreenshot", { format: "png" }).catch(() => null);
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
