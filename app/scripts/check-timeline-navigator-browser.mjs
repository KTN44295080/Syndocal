import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { writeFile, unlink, mkdir } from "node:fs/promises";
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
const fixtureName = `.navigator-proof-${process.pid}.tsx`;
const fixture = resolve(root, fixtureName);
const artifacts = resolve(root, "../target/qa/timeline-navigator-20260905");
await mkdir(artifacts, {recursive:true});
await writeFile(fixture, `
import { createSignal, Show } from 'solid-js';
import { render } from 'solid-js/web';
import { TimelineNavigator } from '/src/components/TimelineNavigator';
import '/src/styles.css';
const [open,setOpen]=createSignal(true),[child,setChild]=createSignal(null),[active,setActive]=createSignal(1);
let resolve; window.proof={calls:[],finish:()=>resolve?.(),setOpen};
const [timelines,setTimelines]=createSignal([{id:1,label:'統合本編'},{id:2,label:'Encore'}]);
const [children,setChildren]=createSignal(Array.from({length:30},(_,i)=>({id:40+i,label:i===0?'DATE':('長いシーン名 '.repeat(8)+i)})));
window.proof.updateArrays=(label)=>{setTimelines(timelines().map(t=>({...t})));setChildren(children().map(c=>({...c,...(c.id===40&&label?{label}:{} )})));};
const selectRoot=async id=>{window.proof.calls.push(['root',id]);setActive(id);setChild(null)};
const selectChild=async id=>{window.proof.calls.push(['child',id]);await new Promise(r=>resolve=r);setChild(id)};
render(()=><main style={{height:'400px',width:'100%','min-width':0,display:'grid','grid-template-rows':'52px minmax(0,1fr)'}}>
<header><button id="toggle" aria-label="タイムライン一覧" aria-expanded={open()} aria-controls="timeline-navigator" onClick={()=>setOpen(!open())}>タイムライン一覧</button></header>
<div class="timelineEditorWithNavigator" classList={{navigatorOpen:open()}} style={{'grid-row':2}}>
<div class="timelineOverviewFrame" data-editor><button style={{height:'44px'}}>既存タイムライン操作</button></div>
<Show when={open()}><TimelineNavigator timelines={timelines()} childCues={children()} activeTimelineId={active()} selectedChildCueId={child()} onSelectRoot={selectRoot} onSelectChild={selectChild} onClose={()=>setOpen(false)}/></Show>
</div></main>,document.body);
`);
const port=5196,origin=`http://127.0.0.1:${port}`;
const vite=spawn(process.execPath,[resolve(root,"node_modules/vite/bin/vite.js"),"--host","127.0.0.1","--port",String(port),"--strictPort"],{cwd:root,windowsHide:true,stdio:["ignore","pipe","pipe"]});
let log="";vite.stdout.on("data",d=>log+=d);vite.stderr.on("data",d=>log+=d);
let browser;
try {
  for(let i=0;;i++){assert.equal(vite.exitCode,null,log);try{if((await fetch(origin)).ok)break;}catch{}assert.ok(i<150,log);await new Promise(r=>setTimeout(r,100));}
  const executablePath=[process.env.CHROME_PATH,"C:/Program Files/Google/Chrome/Application/chrome.exe","C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe"].filter(Boolean).find(existsSync);
  browser=await chromium.launch({headless:true,...(executablePath?{executablePath}:{})});
  for(const width of [1280,640]) {
    const page=await browser.newPage({viewport:{width,height:600}}),errors=[];page.on('pageerror',e=>errors.push(String(e)));
    await page.route('**/navigator-proof',r=>r.fulfill({contentType:'text/html',body:`<html><body><script type="module" src="/${fixtureName}"></script></body></html>`}));
    await page.goto(`${origin}/navigator-proof`);
    await page.locator('[data-timeline-navigator]').waitFor();
    assert.equal(await page.locator('[data-timeline-navigator-root]').count(),2);
    assert.equal(await page.locator('[data-timeline-navigator-child]').count(),30);
    assert.equal(await page.locator('[data-timeline-navigator-root="1"]').getAttribute('aria-current'),'page');
    assert.equal(await page.evaluate(()=>{
      const root=document.querySelector('[data-timeline-navigator-root="1"]'),child=document.querySelector('[data-timeline-navigator-child="40"]');child.focus();
      for(let i=0;i<100;i++)window.proof.updateArrays();
      window.proof.updateArrays('DATE renamed');
      return root===document.querySelector('[data-timeline-navigator-root="1"]')&&child===document.activeElement&&child===document.querySelector('[data-timeline-navigator-child="40"]')&&child.textContent==='DATE renamed';
    }),true);
    const dimensions=await page.evaluate(()=>{const a=document.querySelector('aside').getBoundingClientRect(), e=document.querySelector('[data-editor]').getBoundingClientRect(),s=document.querySelector('.timelineNavigatorScroll');return {right:a.right,left:a.left,editorRight:e.right,editorWidth:e.width,scroll:s.scrollHeight>s.clientHeight,button:document.querySelector('[data-timeline-navigator-child]').getBoundingClientRect().height};});
    assert.ok(dimensions.right<=width+1&&dimensions.left>=dimensions.editorRight-1&&dimensions.editorWidth>0,JSON.stringify(dimensions));
    assert.ok(dimensions.scroll);assert.ok(dimensions.button>=43.5);
    await page.locator('[data-timeline-navigator-child="40"]').click();
    assert.equal(await page.locator('[data-timeline-navigator-root="2"]').isDisabled(),true);
    await page.evaluate(()=>window.proof.finish());
    await page.waitForFunction(()=>document.querySelector('[data-timeline-navigator-child="40"]').getAttribute('aria-current')==='page');
    assert.equal(await page.locator('[data-timeline-navigator-root][aria-current]').count(),0);
    await page.locator('[data-timeline-navigator-root="2"]').click();
    assert.equal(await page.locator('[data-timeline-navigator-root="2"]').getAttribute('aria-current'),'page');
    assert.deepEqual(await page.evaluate(()=>window.proof.calls),[['child',40],['root',2]]);
    await page.screenshot({path:resolve(artifacts,`navigator-${width}.png`)});
    await page.locator('[aria-label="タイムライン一覧を閉じる"]').click();
    assert.equal(await page.locator('[data-timeline-navigator]').count(),0);
    assert.equal(await page.locator('#toggle').getAttribute('aria-expanded'),'false');
    const fullWidth=await page.locator('[data-editor]').evaluate(e=>e.getBoundingClientRect().width);assert.ok(fullWidth>dimensions.editorWidth);
    await page.locator('#toggle').click();await page.locator('[data-timeline-navigator-child="40"]').click();
    await page.locator('[aria-label="タイムライン一覧を閉じる"]').click();
    await page.evaluate(()=>window.proof.finish());await page.locator('#toggle').click();
    assert.equal(await page.locator('[data-timeline-navigator-root="2"]').isDisabled(),false);
    assert.deepEqual(errors,[]);await page.close();
    const appPage=await browser.newPage({viewport:{width,height:720}});
    await appPage.goto(`${origin}/?syndocalViewportFixture=timeline-layered`);
    await appPage.locator('[data-edit-domain-navigation] [data-control-mode-option="live"]').click();
    const toggle=appPage.locator('[data-timeline-navigator-toggle]');await toggle.waitFor();
    if(await toggle.getAttribute('aria-expanded')==='false')await toggle.click();
    await appPage.locator('[data-timeline-navigator]').waitFor();
    assert.ok(await appPage.locator('[data-timeline-navigator-root]').count()>0);
    const actual=await appPage.locator('.timelineEditorWithNavigator').evaluate(el=>{
      const box=el.getBoundingClientRect(),canvas=el.querySelector('.timelineOverviewFrame').getBoundingClientRect(),aside=el.querySelector('aside').getBoundingClientRect();
      return {height:box.height,canvasHeight:canvas.height,canvasWidth:canvas.width,asideRight:aside.right,right:box.right,asideLeft:aside.left,canvasRight:canvas.right};
    });
    assert.ok(actual.height>30&&actual.canvasHeight>30&&actual.canvasWidth>0&&actual.asideRight<=actual.right+1&&actual.asideLeft>=actual.canvasRight-1,JSON.stringify(actual));
    await appPage.screenshot({path:resolve(artifacts,`integrated-${width}.png`)});
    await toggle.click();assert.equal(await appPage.locator('[data-timeline-navigator]').count(),0);
    await appPage.close();
  }
  console.log('Timeline navigator browser checks passed: 1280/640 containment, internal scrolling, unchanged controls, root/child selection, pending exclusion, collapse and pending-unmount cleanup.');
} finally {await browser?.close();vite.kill();await unlink(fixture).catch(()=>{});}
