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
const fixtureName = `.inline-child-proof-${process.pid}.tsx`;
const fixture = resolve(root, fixtureName);
const artifacts = resolve(root, "../target/qa/timeline-inline-child-20260905");
await mkdir(artifacts, {recursive:true});
await writeFile(fixture, `
import { createSignal, Show } from 'solid-js';
import { render } from 'solid-js/web';
import { cueIdentityCss } from '/src/identityColor';
import { TimelineInlineChildCanvas, TimelineInlineChildGutters } from '/src/components/TimelineInlineChildRows';
const [expanded,setExpanded]=createSignal(false);
const [start,setStart]=createSignal(1000);
const [identities,setIdentities]=createSignal({
  40:{color:'#3366cc',groupId:'cool',groupColor:'#e54c72'},
  41:{color:'#c93f5c',groupId:'warm',groupColor:'#31b67b'},
});
window.proof={opens:[],edits:0,setStart,setIdentities,identities,
  identityCss:(id,role)=>{const identity=identities()[id];return cueIdentityCss(id,identity?.color,role,identity?.groupId,identity?.groupColor);}};
const rows=[{key:'1:2',parentEventId:1,parentCueId:20,parentLabel:'DATE',label:'照明 Color',muted:false,
top:30,height:60,railCount:2,issue:null,blocks:[{key:'1',cueId:40,label:'Blue',startMs:1000,endMs:2000,rail:0},
{key:'2',cueId:41,label:'White',startMs:1500,endMs:2500,rail:1}]}];
const open=id=>window.proof.opens.push(id);
render(()=><main style={{display:'flex',width:'100%',color:'#eee',background:'#101820'}}>
<aside style={{position:'relative',width:'200px',height:'120px','flex-shrink':0}}>
<button id="expand" onClick={()=>setExpanded(!expanded())} aria-expanded={expanded()}>→ DATE</button>
<Show when={expanded()}><TimelineInlineChildGutters rows={rows} sectionTop={0} onOpen={open}/></Show></aside>
<svg viewBox="0 0 600 120" style={{width:'600px','max-width':'calc(100% - 200px)'}} onPointerDown={()=>window.proof.edits++} onDrop={()=>window.proof.edits++}>
<Show when={expanded()}><TimelineInlineChildCanvas rows={rows} width={600} visibleStartMs={start()} visibleEndMs={start()+3000} laneHeight={30} cueIdentities={identities()} onOpen={open}/></Show>
</svg></main>, document.body);
`);
const port=5197, origin=`http://127.0.0.1:${port}`;
const vite=spawn(process.execPath,[resolve(root,"node_modules/vite/bin/vite.js"),"--host","127.0.0.1","--port",String(port),"--strictPort"],{cwd:root,windowsHide:true,stdio:["ignore","pipe","pipe"]});
let log=""; vite.stdout.on("data",d=>log+=d); vite.stderr.on("data",d=>log+=d);
let browser;
try {
  for(let i=0;;i++) { assert.equal(vite.exitCode,null,log); try {if((await fetch(origin)).ok) break;}catch{} assert.ok(i<150,log); await new Promise(r=>setTimeout(r,100)); }
  const executablePath=[
    process.env.CHROME_PATH,
    process.env.EDGE_PATH,
    process.env.LOCALAPPDATA ? resolve(process.env.LOCALAPPDATA,"Google/Chrome/Application/chrome.exe") : null,
    process.env.LOCALAPPDATA ? resolve(process.env.LOCALAPPDATA,"Microsoft/Edge/Application/msedge.exe") : null,
    "C:/Program Files/Google/Chrome/Application/chrome.exe",
    "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe",
  ].filter(Boolean).find(existsSync);
  browser=await chromium.launch({headless:true,...(executablePath?{executablePath}:{})});
  for(const width of [1280,640]) {
    const page=await browser.newPage({viewport:{width,height:480}}); const errors=[]; page.on('pageerror',e=>errors.push(String(e)));
    await page.route('**/inline-proof',r=>r.fulfill({contentType:'text/html',body:`<html><body><script type="module" src="/${fixtureName}"></script></body></html>`}));
    await page.goto(`${origin}/inline-proof`); await page.locator('#expand').click();
    assert.equal(await page.locator('.timelineInlineChildBlock').count(),2);
    const rects=await page.locator('.timelineInlineChildBlock').evaluateAll(els=>els.map(el=>({x:el.getAttribute('x'),y:el.getAttribute('y'),width:el.getAttribute('width')})));
    assert.deepEqual(rects,[{x:'0',y:'32',width:'200'},{x:'100',y:'62',width:'200'}]);
    const readColors=()=>page.evaluate(()=>{
      const probe=document.createElementNS('http://www.w3.org/2000/svg','svg');
      probe.style.position='absolute';probe.style.left='-9999px';
      const sample=document.createElementNS(probe.namespaceURI,'rect');probe.append(sample);document.body.append(probe);
      const normalize=value=>{sample.style.fill=value;return getComputedStyle(sample).fill;};
      const results=[...document.querySelectorAll('.timelineInlineChildBlock')].map((block,index)=>{
        const id=[40,41][index];const css=getComputedStyle(block);
        return {actual:{fill:css.fill,band:css.stroke,text:getComputedStyle(block.parentElement.querySelector('text')).fill},
          expected:Object.fromEntries(['fill','band','text'].map(role=>[role,normalize(window.proof.identityCss(id,role))]))};
      });probe.remove();return results;
    });
    const assertColors=async label=>{const values=await readColors();for(const value of values)assert.deepEqual(value.actual,value.expected,label);return values.map(value=>value.actual);};
    const initialColors=await assertColors('persisted cue colors match the shared identity helper for fill/band/text');
    assert.notDeepEqual(initialColors[0],initialColors[1],'different authored cue colors must remain visually distinct');
    await page.evaluate(()=>{window.proof.initialBlocks=[...document.querySelectorAll('.timelineInlineChildBlock')];window.proof.setIdentities(current=>({...current,41:{...current[41],color:null}}));});
    const groupColors=await assertColors('missing cue color uses the owning group color');
    assert.notDeepEqual(groupColors[1],initialColors[1],'group fallback visibly replaces the removed cue color');
    assert.deepEqual(groupColors[0],initialColors[0],'another cue retains its own color');
    await page.evaluate(()=>window.proof.setIdentities(current=>({...current,40:{...current[40],color:'#a84fd1'},41:{...current[41],groupColor:'#e5b633'}})));
    const changedColors=await assertColors('cue and group color signals update all three rendered roles');
    assert.notDeepEqual(changedColors[0],groupColors[0]);assert.notDeepEqual(changedColors[1],groupColors[1]);
    assert.equal(await page.evaluate(()=>window.proof.initialBlocks.every((block,index)=>block===document.querySelectorAll('.timelineInlineChildBlock')[index])),true,'color changes retain the existing block DOM');
    await page.locator('.timelineInlineChildBlock').first().click();
    assert.equal(await page.evaluate(()=>window.proof.edits),0);
    await page.locator('.timelineInlineChildBlock').first().dblclick();
    assert.deepEqual(await page.evaluate(()=>window.proof.opens),[20]);
    await page.locator('[data-timeline-inline-child-gutter]').focus(); await page.keyboard.press('Enter');
    assert.deepEqual(await page.evaluate(()=>window.proof.opens),[20,20]);
    await page.evaluate(()=>window.proof.setStart(1800));
    assert.equal(await page.locator('.timelineInlineChildBlock').first().getAttribute('width'),'40');
    await page.evaluate(()=>window.proof.setStart(2000));
    assert.equal(await page.locator('.timelineInlineChildBlock').count(),1);
    await page.screenshot({path:resolve(artifacts,`readonly-child-${width}.png`)});
    await page.locator('#expand').click(); assert.equal(await page.locator('[data-timeline-inline-child]').count(),0);
    assert.deepEqual(errors,[]); await page.close();
  }
  console.log('Inline child browser checks passed: two viewports, shared cue fill/band/text, group-color fallback, reactive color updates without block replacement, overlap rails, viewport clipping, readonly pointer isolation, drill-in keyboard/double-click and collapse.');
} finally { await browser?.close(); vite.kill(); await unlink(fixture).catch(()=>{}); }
