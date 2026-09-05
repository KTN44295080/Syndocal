import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { writeFile, unlink, readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { homedir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
const require = createRequire(import.meta.url);
let chromium;
try { ({chromium} = require("playwright")); } catch {
  ({chromium} = require(resolve(homedir(), ".cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright")));
}
const root=resolve(dirname(fileURLToPath(import.meta.url)),"..");
const name=`.context-proof-${process.pid}.tsx`,fixture=resolve(root,name);
const source=await readFile(resolve(root,'src/components/MappingPersistentWorkspaceBand.tsx'),'utf8');
assert.match(source, /<MappingWorkspaceContextContent contextContent=\{props.contextContent\}/);
await writeFile(fixture, `
import {createSignal,Show,onCleanup} from 'solid-js';import {render} from 'solid-js/web';
import {MappingWorkspaceContextContent} from '/src/components/MappingPersistentWorkspaceBand';
const [open,setOpen]=createSignal(true),[context,setContext]=createSignal(1),[keep,setKeep]=createSignal(true);
window.proof={made:[],cleaned:[],childMade:0,childCleaned:0,setOpen,setContext,setKeep};
function Context(props){const id=props.id;window.proof.made.push(id);onCleanup(()=>window.proof.cleaned.push(id));return <div data-context={id}>Context {id}</div>}
function Child(){window.proof.childMade++;onCleanup(()=>window.proof.childCleaned++);return <div data-child>Background</div>}
render(()=><Show when={open()}><MappingWorkspaceContextContent contextContent={(() => {const id=context();return id===null?undefined:<Context id={id}/>;})()} keepChildrenMounted={keep()}><Child/></MappingWorkspaceContextContent></Show>,document.body);
`);
const port=5198,origin=`http://127.0.0.1:${port}`;
const vite=spawn(process.execPath,[resolve(root,'node_modules/vite/bin/vite.js'),'--host','127.0.0.1','--port',String(port),'--strictPort'],{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe']});
let log='';vite.stdout.on('data',d=>log+=d);vite.stderr.on('data',d=>log+=d);let browser;
try {
  for(let i=0;;i++){assert.equal(vite.exitCode,null,log);try{if((await fetch(origin)).ok)break;}catch{}assert(i<150,log);await new Promise(r=>setTimeout(r,100));}
  const executablePath=['C:/Program Files/Google/Chrome/Application/chrome.exe','C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe'].find(existsSync);
  browser=await chromium.launch({headless:true,...(executablePath?{executablePath}:{})});
  const page=await browser.newPage(),errors=[];page.on('pageerror',e=>errors.push(String(e)));
  await page.route('**/context-proof',r=>r.fulfill({contentType:'text/html',body:'<html><body><script type="module" src="/'+name+'"></script></body></html>'}));
  await page.goto(origin+'/context-proof');await page.locator('[data-context]').waitFor();
  const counts=()=>page.evaluate(()=>({made:window.proof.made,cleaned:window.proof.cleaned,childMade:window.proof.childMade,childCleaned:window.proof.childCleaned}));
  assert.deepEqual(await counts(),{made:[1],cleaned:[],childMade:1,childCleaned:0},'One displayed tree and one intended hidden child; no invisible second context');
  assert.equal(await page.locator('[data-persistent-band-background-content] [data-child]').count(),1);
  await page.evaluate(()=>window.proof.setKeep(false));
  assert.deepEqual(await counts(),{made:[1],cleaned:[],childMade:1,childCleaned:1},'Presence check cannot recreate context when background toggle changes');
  await page.evaluate(()=>window.proof.setKeep(true));
  await page.evaluate(()=>window.proof.setContext(2));
  assert.deepEqual((await counts()).made,[1,2]);assert.deepEqual((await counts()).cleaned,[1]);
  assert.equal(await page.locator('[data-context="2"]').count(),1);
  await page.evaluate(()=>window.proof.setContext(null));
  assert.deepEqual((await counts()).cleaned,[1,2]);
  assert.equal(await page.locator('[data-context]').count(),0);assert.equal(await page.locator('[data-child]').count(),1);
  assert.equal(await page.locator('[data-child]').isVisible(),true,'Absent context displays children as the fallback');
  await page.evaluate(()=>window.proof.setContext(3));
  assert.deepEqual((await counts()).made,[1,2,3]);
  await page.evaluate(()=>window.proof.setOpen(false));
  const final=await counts();assert.deepEqual(final.cleaned,[1,2,3]);assert.equal(final.childMade,final.childCleaned,'Every intended child owner is cleaned on unmount');
  assert.equal(await page.locator('[data-context],[data-child]').count(),0);assert.deepEqual(errors,[]);
  console.log('PASS actual workspace context slot: one JSX getter construction, no duplicate side-effect owner, replacement cleanup, hidden background toggling, fallback and final disposal.');
} finally {await browser?.close();vite.kill();await unlink(fixture).catch(()=>{});}
