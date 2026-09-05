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
const [outputs, setOutputs] = createSignal([
  {id: 1, label: 'Foreground', width: 1920, height: 1080, enabled: true, blackout: false},
  {id: 2, label: 'Background', width: 1080, height: 1920, enabled: true, blackout: false},
]);
window.proof = {calls: [], setVisible, setOutputs};
const invoke = async (command, args) => {
  if (command !== 'get_live_video_monitor_frame') throw new Error(command);
  window.proof.calls.push(args);
  const canvas = document.createElement('canvas'); canvas.width = args.width; canvas.height = args.height;
  const ctx = canvas.getContext('2d'); ctx.fillStyle = args.outputId === 1 ? '#c64372' : '#307bbe';
  ctx.fillRect(0,0,canvas.width,canvas.height);
  const jpeg = Uint8Array.from(atob(canvas.toDataURL('image/jpeg').split(',')[1]), c => c.charCodeAt(0));
  const packet = new Uint8Array(40+jpeg.length); packet.set([83,89,76,86,1,0,0,0]);
  const view = new DataView(packet.buffer); view.setUint16(32,args.width,true); view.setUint16(34,args.height,true);
  view.setUint32(36,jpeg.length,true); packet.set(jpeg,40); return packet;
};
render(() => <main style={{width:'600px', 'max-width':'100%', padding:'12px'}}>
  <Show when={visible()}><TimelineOutputPreview outputs={outputs()} invoke={invoke} backendAvailable={true}/></Show>
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
    await page.route('**/output-proof', route => route.fulfill({contentType:'text/html',body:`<!doctype html><html><body><script type="module" src="/${fixtureName}"></script></body></html>`}));
    await page.goto(`${origin}/output-proof`);
    await page.waitForFunction(() => [...document.querySelectorAll('[data-timeline-output-id] img')].filter(x=>x.naturalWidth>0).length===2);
    const dimensions = await page.locator('.timelineOutputPreviewViewport').evaluateAll(els=>els.map(el=>{const r=el.getBoundingClientRect();return r.width/r.height;}));
    assert.ok(Math.abs(dimensions[0]-16/9)<0.01 && Math.abs(dimensions[1]-9/16)<0.01, JSON.stringify(dimensions));
    const calls = await page.evaluate(()=>window.proof.calls);
    assert.deepEqual([...new Set(calls.map(c=>c.outputId))].sort(),[1,2]);
    assert.ok(calls.every(c=>c.monitorKind==='program' && c.width<=320 && c.height<=180));
    await page.screenshot({path:resolve(artifacts,`outputs-${viewport.width}.png`)});
    await page.evaluate(()=>window.proof.setVisible(false));
    await sleep(150); const closedCount=await page.evaluate(()=>window.proof.calls.length);
    await sleep(350); assert.equal(await page.evaluate(()=>window.proof.calls.length),closedCount);
    assert.deepEqual(errors,[]);
    console.log(`PASS Timeline tabs and all output preview ${viewport.width}: two actual ratios, IPC stops on unmount`);
    await page.close();
  }
} finally {
  await browser?.close(); vite.kill(); await unlink(fixture);
}
