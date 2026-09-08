import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { writeFile, unlink, mkdir, readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
const require = createRequire(import.meta.url);
let chromium;
try { ({chromium} = require("playwright")); } catch {
  ({chromium} = require(resolve(homedir(), ".cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright")));
}
const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const fixtureName = `.bpm-proof-${process.pid}.tsx`, fixture = resolve(root, fixtureName);
const artifacts = resolve(root, "../target/qa/topbar-bpm-20260905");
await mkdir(artifacts, {recursive:true});
const chrome = await readFile(resolve(root, "src/components/WorkspaceChrome.tsx"), "utf8");
assert.match(chrome, /<TopbarBpmControl bpm=\{props\.bpm\} onSetBpm=\{props\.onSetBpm\}/);
await writeFile(fixture, `
import {createSignal,Show} from 'solid-js'; import {render} from 'solid-js/web';
import {TopbarBpmControl} from '/src/components/TopbarBpmControl'; import '/src/styles.css';
const [bpm,setBpm]=createSignal(120),[open,setOpen]=createSignal(true);
window.proof={calls:[],mode:'ok',drag:0,setBpm,setOpen,finish:null};
const apply=async value=>{window.proof.calls.push(value);if(window.proof.mode==='reject')throw Error('backend rejected');
if(window.proof.mode==='pending')await new Promise(resolve=>window.proof.finish=resolve);setBpm(value)};
render(()=><header class="topbar" data-tauri-drag-region onMouseDown={()=>window.proof.drag++}>
<div class="status"><Show when={open()}><TopbarBpmControl bpm={bpm()} onSetBpm={apply}/></Show><button id="outside">Outside</button></div></header>,document.body);
`);
const port=5197,origin=`http://127.0.0.1:${port}`;
const vite=spawn(process.execPath,[resolve(root,"node_modules/vite/bin/vite.js"),"--host","127.0.0.1","--port",String(port),"--strictPort"],{cwd:root,windowsHide:true,stdio:["ignore","pipe","pipe"]});
let log="";vite.stdout.on("data",d=>log+=d);vite.stderr.on("data",d=>log+=d);
let browser;
try {
  for(let i=0;;i++){assert.equal(vite.exitCode,null,log);try{if((await fetch(origin)).ok)break;}catch{}assert(i<150,log);await new Promise(r=>setTimeout(r,100));}
  const executablePath=[
    process.env.CHROME_PATH,
    process.env.EDGE_PATH,
    process.env.LOCALAPPDATA ? resolve(process.env.LOCALAPPDATA,"Google/Chrome/Application/chrome.exe") : null,
    process.env.LOCALAPPDATA ? resolve(process.env.LOCALAPPDATA,"Microsoft/Edge/Application/msedge.exe") : null,
    "C:/Program Files/Google/Chrome/Application/chrome.exe",
    "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
  ].filter(Boolean).find(existsSync);
  browser=await chromium.launch({headless:true,...(executablePath?{executablePath}:{})});
  const page=await browser.newPage({viewport:{width:1280,height:720}}), errors=[];
  page.on('pageerror',error=>errors.push(String(error)));
  await page.route('**/bpm-proof',r=>r.fulfill({contentType:'text/html',body:`<html><body><script type="module" src="/${fixtureName}"></script></body></html>`}));
  await page.goto(`${origin}/bpm-proof`);
  const value=page.getByRole('button',{name:'Edit BPM'}), input=page.getByRole('spinbutton',{name:'BPM'});
  await value.click(); await input.waitFor();
  assert.equal(await input.evaluate(el=>document.activeElement===el),true);
  assert.equal(await page.locator('.topbarBpmControl [data-tauri-drag-region]').count(),0);
  assert.equal(await page.evaluate(()=>window.proof.drag),0);
  await input.fill('137.5'); await input.press('Enter'); await value.waitFor();
  assert.deepEqual(await page.evaluate(()=>window.proof.calls),[137.5]);
  await value.dblclick(); await input.fill('140'); await input.press('Escape'); await value.waitFor();
  assert.deepEqual(await page.evaluate(()=>window.proof.calls),[137.5]);
  await page.locator('.topbarBpmControl small').click(); await input.fill('145'); await page.locator('#outside').click(); await value.waitFor();
  assert.deepEqual(await page.evaluate(()=>window.proof.calls),[137.5,145]);
  await value.click(); await input.fill('301'); await input.press('Enter');
  assert.equal(await input.getAttribute('aria-invalid'),'true'); assert.equal(await page.getByRole('alert').count(),1);
  await input.fill(''); await input.press('Enter'); assert.deepEqual(await page.evaluate(()=>window.proof.calls),[137.5,145]);
  await input.fill('150'); await page.evaluate(()=>window.proof.setBpm(149));
  assert.equal(await input.inputValue(),'150','snapshot publication must preserve active draft');
  await page.evaluate(()=>window.proof.mode='reject'); await input.press('Enter');
  await page.waitForFunction(()=>document.querySelector('[role=alert]')?.textContent.includes('backend rejected'));
  assert.equal(await input.inputValue(),'150'); assert.equal(await input.isEnabled(),true);
  assert.equal(await page.getByRole('alert').evaluate(el=>{const r=el.getBoundingClientRect();return r.top>=28&&r.left>=0&&r.right<=innerWidth&&r.height>20;}),true,'error escapes clipped native titlebar');
  await page.screenshot({path:resolve(artifacts,'rejected-1280.png')});
  await page.evaluate(()=>window.proof.setBpm(150)); await input.press('Enter'); await value.waitFor();
  assert.equal(await page.getByRole('alert').count(),0,'Same-value completion clears prior rejection');
  assert.deepEqual(await page.evaluate(()=>window.proof.calls),[137.5,145,150],'Same-value retry does not send another mutation');
  await value.click(); await input.fill('151');
  await page.evaluate(()=>window.proof.mode='pending'); await input.press('Enter');
  assert.equal(await input.isDisabled(),true);
  assert.deepEqual(await page.evaluate(()=>window.proof.calls),[137.5,145,150,151],'Enter-triggered disable/blur cannot duplicate commit');
  await page.evaluate(()=>window.proof.finish()); await value.waitFor();
  await page.screenshot({path:resolve(artifacts,'readout-1280.png')});
  await value.click(); await input.fill('160'); await input.press('Enter');
  await page.evaluate(()=>{window.proof.setOpen(false);window.proof.finish();});
  await page.waitForTimeout(30); assert.deepEqual(errors,[]);
  console.log('PASS actual topbar BPM component: click/double-click/label editing, focus/no drag, Enter/blur/Escape, bounds, preserved draft, rejection, one pending commit and unmount safety.');
} finally {await browser?.close();vite.kill();await unlink(fixture).catch(()=>{});}
