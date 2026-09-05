import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { mkdir, writeFile, unlink } from "node:fs/promises";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const require = createRequire(import.meta.url);
const { chromium } = (() => {
  try { return require("playwright"); } catch {
    return require(process.env.PLAYWRIGHT_MODULE_PATH ?? resolve(homedir(),
      ".cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright"));
  }
})();
const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const artifacts = resolve(appRoot, "../target/qa/timeline-output-preview-20260905");
const fixtureName = `.timeline-output-preview-${process.pid}.tsx`;
const fixture = resolve(appRoot, fixtureName);
const port = Number(process.env.TIMELINE_OUTPUT_PREVIEW_PORT ?? 5198);
const origin = `http://127.0.0.1:${port}`;
const sleep = (ms) => new Promise((done) => setTimeout(done, ms));
const chromePath = [process.env.CHROME_PATH, "C:/Program Files/Google/Chrome/Application/chrome.exe",
  "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"].find((path) => path && existsSync(path));
await mkdir(artifacts, { recursive: true });
// Real Solid component and controller, mocked read-only monitor IPC. No physical output.
await writeFile(fixture, `
import { createSignal, Show } from 'solid-js';
import { render } from 'solid-js/web';
import { TimelineOutputPreview } from '/src/components/TimelineOutputPreview';
import '/src/styles.css';
const [visible, setVisible] = createSignal(true);
const [epoch,setEpoch]=createSignal(1);
const [outputs, setOutputs] = createSignal([
  {id: 1, label: 'Foreground', width: 1920, height: 1080, enabled: true, blackout: false},
  {id: 2, label: 'Background', width: 1080, height: 1920, enabled: true, blackout: false},
]);
window.proof = {calls: [], setVisible, setOutputs, setEpoch, mode:"frame", pending:[]};
URL.createObjectURL=()=>{throw new Error("Timeline must not create Blob URLs");};
const invoke = async (command, args) => {
  if (command !== 'get_live_video_monitor_frame') throw new Error(command);
  window.proof.calls.push(args);
  if(args.pixelFormat!=='rgba')throw new Error('Timeline must request RGBA');
  const busy=window.proof.mode==='busy';
  const length=busy?0:args.width*args.height*4;
  const packet=new Uint8Array(40+length);packet.set([83,89,76,86,2,busy?1:0,0,1]);
  const view=new DataView(packet.buffer);view.setUint16(32,args.width,true);view.setUint16(34,args.height,true);
  view.setUint32(36,length,true);
  const pixel=args.outputId===1?[198,67,114,255]:[48,123,190,255];
  for(let offset=40;offset<packet.length;offset+=4)packet.set(pixel,offset);
  if(window.proof.mode==='pending')return new Promise(resolve=>window.proof.pending.push(()=>resolve(packet)));
  return packet;
};
render(() => <main class="layoutSharedWorkspace layoutControl controlModeLive" style={{width:'min(1080px, 80vw)', height:'45vh', padding:'12px'}}>
  <div class="controlContextPane" style={{height:'100%', display:'grid'}}>
  <section class="timelineExternalSourceShelf">
    <header class="timelineExternalSourceShelfHeader"><h3>Video Preview</h3></header>
    <section id="timeline-source-context-panel-video-preview">
      <Show when={visible()}><TimelineOutputPreview outputs={outputs()} invoke={invoke} backendAvailable={true} projectEpoch={epoch()}/></Show>
    </section>
  </section></div>
</main>, document.body);
`);
const vite = spawn(process.execPath, [resolve(appRoot, "node_modules/vite/bin/vite.js"), "--host", "127.0.0.1", "--port", String(port), "--strictPort"],
  { cwd: appRoot, windowsHide: true, stdio: ["ignore", "pipe", "pipe"] });
let viteLog = "";
vite.stdout.on("data", (data) => { viteLog += data; });
vite.stderr.on("data", (data) => { viteLog += data; });
let browser;
try {
  for (let i = 0; ; i++) {
    assert.equal(vite.exitCode, null, viteLog);
    try { if ((await fetch(origin)).ok) break; } catch {}
    assert.ok(i < 150, viteLog); await sleep(100);
  }
  browser = await chromium.launch({headless:true, ...(chromePath ? {executablePath:chromePath} : {})});
  for (const viewport of [{width:1920,height:1080},{width:1280,height:720}]) {
    const page = await browser.newPage({viewport});
    const errors = []; page.on('pageerror', e => errors.push(String(e)));
    await page.addInitScript(() => localStorage.setItem('syndocal.uiLocale.v1','en'));
    if (process.env.OUTPUT_PREVIEW_COMPONENT_ONLY !== '1') {
    await page.goto(`${origin}/?syndocalViewportFixture=timeline-layered`);
    await page.locator('[data-edit-domain-navigation] [data-control-mode-option="live"]').click();
    const tabs = page.locator('[data-timeline-source-shelf-mode]');
    await tabs.first().waitFor().catch(async error => {
      console.error(errors, (await page.locator('body').innerText()).slice(-7000));
      await page.screenshot({path:resolve(artifacts,'failure.png')}); throw error;
    }); assert.equal(await tabs.count(),3);
    await tabs.first().click();
    assert.deepEqual(await page.locator('[data-timeline-source-shelf-filter]').allTextContents(),['All','Lighting','Video','Audio']);
    await page.locator('[data-timeline-source-shelf-filter="lighting"]').click();
    assert.equal(await page.locator('[data-timeline-source-shelf-media]').count(),0);
    await page.locator('[data-timeline-source-shelf-filter="all"]').click();
    assert.equal(await page.locator('[data-timeline-source-shelf-scenes]').count(),1);
    assert.equal(await page.locator('[data-timeline-source-shelf-media]').count(),1);
    await tabs.first().focus(); await page.keyboard.press('End');
    assert.equal(await tabs.last().getAttribute('aria-selected'),'true');
    await page.locator('[data-timeline-output-preview]').waitFor();
    assert.equal(await page.locator('[data-timeline-source-shelf-filter]').count(),0);
    const geometry = await tabs.evaluateAll(items => items.map(el => {const r=el.getBoundingClientRect();return {height:r.height,left:r.left,right:r.right};}));
    assert.ok(geometry.every(r => r.height >= 43.5 && r.left >= 0 && r.right <= viewport.width), JSON.stringify(geometry));
    await page.screenshot({path:resolve(artifacts,`tabs-${viewport.width}.png`)});
    await tabs.last().focus(); await page.keyboard.press('Home');
    assert.equal(await page.locator('[data-timeline-output-preview]').count(),0);
    assert.deepEqual(errors,[]);
    }
    await page.route('**/output-proof', route => route.fulfill({contentType:'text/html',body:`<!doctype html><html><body><script type="module" src="/${fixtureName}"></script></body></html>`}));
    await page.goto(`${origin}/output-proof`);
    await page.waitForFunction(() => [...document.querySelectorAll('[data-timeline-output-id] canvas')].filter(x=>getComputedStyle(x).visibility==='visible').length===2);
    const dimensions = await page.locator('.timelineOutputPreviewViewport').evaluateAll(els=>els.map(el=>{const r=el.getBoundingClientRect();return r.width/r.height;}));
    assert.ok(Math.abs(dimensions[0]-16/9)<0.01 && Math.abs(dimensions[1]-9/16)<0.01, JSON.stringify(dimensions));
    const calls = await page.evaluate(()=>window.proof.calls);
    assert.deepEqual([...new Set(calls.map(c=>c.outputId))].sort(),[1,2]);
    assert.ok(calls.every(c=>c.monitorKind==='program' && c.pixelFormat==='rgba' && c.width<=320 && c.height<=180));
    const readPixels=()=>page.locator('.timelineOutputPreviewViewport canvas').evaluateAll(elements=>elements.map(canvas=>[...canvas.getContext('2d').getImageData(0,0,1,1).data]));
    assert.deepEqual(await readPixels(),[[198,67,114,255],[48,123,190,255]],'raw RGBA channels render exactly without JPEG conversion');
    await page.evaluate(()=>{window.proof.canvases=[...document.querySelectorAll('.timelineOutputPreviewViewport canvas')];window.proof.mode='busy';});
    await sleep(100);
    assert.deepEqual(await readPixels(),[[198,67,114,255],[48,123,190,255]],'brief BUSY preserves current pixels');
    await page.waitForFunction(()=>[...document.querySelectorAll('.timelineOutputPreviewViewport canvas')].every(canvas=>getComputedStyle(canvas).visibility==='hidden'));
    assert.deepEqual(await readPixels(),[[0,0,0,0],[0,0,0,0]],'expiry clears actual canvas storage, not just a label');
    await page.evaluate(()=>window.proof.mode='frame');
    await page.waitForFunction(()=>[...document.querySelectorAll('.timelineOutputPreviewViewport canvas')].every(canvas=>getComputedStyle(canvas).visibility==='visible'));
    assert.equal(await page.evaluate(()=>window.proof.canvases.every((canvas,index)=>canvas===document.querySelectorAll('.timelineOutputPreviewViewport canvas')[index])),true,'frame recovery keeps the same canvas nodes');
    await page.evaluate(()=>window.proof.mode='pending');
    await page.waitForFunction(()=>window.proof.pending.length>0);
    await page.evaluate(()=>window.proof.setEpoch(2));
    assert.deepEqual(await readPixels(),[[0,0,0,0],[0,0,0,0]],'project change immediately clears both canvases');
    await page.evaluate(()=>window.proof.pending.shift()());
    await sleep(75);
    assert.deepEqual(await readPixels(),[[0,0,0,0],[0,0,0,0]],'retired in-flight response cannot repaint cleared canvases');
    await page.evaluate(()=>{window.proof.mode='frame';for(const resolve of window.proof.pending.splice(0))resolve();});
    await page.waitForFunction(()=>[...document.querySelectorAll('.timelineOutputPreviewViewport canvas')].every(canvas=>getComputedStyle(canvas).visibility==='visible'));
    await page.screenshot({path:resolve(artifacts,`outputs-${viewport.width}.png`)});
    for (const ratios of [[2.5], [2.5,2.5], [16/9,9/16], [2.5,16/9,9/16,1,2.5,16/9]]) {
      await page.evaluate(ratios=>window.proof.setOutputs(ratios.map((ratio,index)=>({
        id:index+1,label:`Output ${index+1}`,width:Math.round(720*ratio),height:720,enabled:true,blackout:false,
      }))),ratios);
      await page.waitForFunction(count=>document.querySelectorAll('.timelineOutputPreviewViewport').length===count,ratios.length);
      await page.waitForFunction(()=>[...document.querySelectorAll('.timelineOutputPreviewViewport canvas')].every(canvas=>getComputedStyle(canvas).visibility==='visible'));
      const boxes=await page.locator('.timelineOutputPreviewViewport').evaluateAll(elements=>elements.map(el=>{
        const r=el.getBoundingClientRect(), area=el.parentElement.getBoundingClientRect();
        return {width:r.width,height:r.height,areaWidth:area.width,areaHeight:area.height,left:r.left,right:r.right};
      }));
      boxes.forEach((box,index)=>{
        assert.ok(box.width>0 && box.height>0,JSON.stringify(box));
        assert.ok(Math.abs(box.width/box.height-ratios[index])<0.015,JSON.stringify({box,ratio:ratios[index]}));
        assert.ok(box.width<=box.areaWidth+1 && box.height<=box.areaHeight+1,JSON.stringify(box));
        assert.ok(box.left>=0 && box.right<=viewport.width,JSON.stringify(box));
        const ideal=Math.min(box.areaWidth,box.areaHeight*ratios[index]);
        assert.ok(Math.abs(box.width-ideal)<2,'preview must fill the available width or height without the old 180px cap');
      });
      const containment=await page.locator('#timeline-source-context-panel-video-preview').evaluate(el=>({
        client:el.clientWidth,scroll:el.scrollWidth,bottom:el.getBoundingClientRect().bottom,
      }));
      assert.ok(containment.scroll<=containment.client+1 && containment.bottom<=viewport.height,JSON.stringify(containment));
      await page.locator('[data-timeline-output-id]').last().scrollIntoViewIfNeeded();
      const lastVisible=await page.locator('[data-timeline-output-id]').last().evaluate(el=>{
        const r=el.getBoundingClientRect(), panel=document.querySelector('#timeline-source-context-panel-video-preview').getBoundingClientRect();
        return r.top>=panel.top-1 && r.bottom<=panel.bottom+1;
      });
      assert.ok(lastVisible,'every output remains reachable inside the preview panel');
      await page.locator('#timeline-source-context-panel-video-preview').evaluate(el=>el.scrollTop=0);
      await page.screenshot({path:resolve(artifacts,`sizing-${viewport.width}-${ratios.length}-${ratios[0]}.png`)});
    }
    await page.evaluate(()=>window.proof.setVisible(false));
    await sleep(150); const closedCount=await page.evaluate(()=>window.proof.calls.length);
    await sleep(350); assert.equal(await page.evaluate(()=>window.proof.calls.length),closedCount);
    assert.deepEqual(errors,[]);
    console.log(`PASS Timeline ${process.env.OUTPUT_PREVIEW_COMPONENT_ONLY === "1" ? "component-only" : "tabs and"} output preview ${viewport.width}: raw pixels, busy retention/expiry clear, generation rejection, persistent canvases, available-area sizing for 1/2/6 mixed-aspect outputs, scroll reachability and unmount`);
    await page.close();
  }
} finally {
  await browser?.close(); vite.kill(); await unlink(fixture);
}
