import assert from "node:assert/strict";
import {spawn} from "node:child_process";
import {existsSync} from "node:fs";
import {writeFile,unlink,mkdir} from "node:fs/promises";
import {createRequire} from "node:module";
import {homedir} from "node:os";
import {dirname,resolve} from "node:path";
import {fileURLToPath} from "node:url";
const require=createRequire(import.meta.url);let chromium;
try{({chromium}=require('playwright'));}catch{({chromium}=require(resolve(homedir(),'.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright')));}
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..'),name=`.orientation-proof-${process.pid}.tsx`,fixture=resolve(root,name);
const artifacts=resolve(root,'../target/qa/mapping-orientation-20260905');await mkdir(artifacts,{recursive:true});
await writeFile(fixture,`
import {createSignal} from 'solid-js';import {render} from 'solid-js/web';
import {MappingSelectionActionsPanel} from '/src/components/MappingSelectionActionsPanel';import '/src/styles.css';
const [count,setCount]=createSignal(2);window.proof={calls:[],setCount,finish:null};
render(()=><main style={{width:'min(100%, 320px)'}}><MappingSelectionActionsPanel selectedCount={count()} flagState={{count:count(),allHighlighted:false,allSoloed:false,allParked:false}}
snapSize={1} hasSelectedStageObject={false} onMatchOrientations={async()=>{window.proof.calls.push('match');await new Promise(r=>window.proof.finish=r)}}
onAimInstallationAxes={async point=>{window.proof.calls.push(point)}}/></main>,document.body);
`);
const port=5199,origin=`http://127.0.0.1:${port}`;
const vite=spawn(process.execPath,[resolve(root,'node_modules/vite/bin/vite.js'),'--host','127.0.0.1','--port',String(port),'--strictPort'],{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe']});
let log='';vite.stdout.on('data',d=>log+=d);vite.stderr.on('data',d=>log+=d);let browser;
try{
for(let i=0;;i++){assert.equal(vite.exitCode,null,log);try{if((await fetch(origin)).ok)break;}catch{}assert(i<150,log);await new Promise(r=>setTimeout(r,100));}
const executablePath=[
process.env.CHROME_PATH,
process.env.EDGE_PATH,
process.env.LOCALAPPDATA ? resolve(process.env.LOCALAPPDATA,'Google/Chrome/Application/chrome.exe') : null,
process.env.LOCALAPPDATA ? resolve(process.env.LOCALAPPDATA,'Microsoft/Edge/Application/msedge.exe') : null,
'C:/Program Files/Google/Chrome/Application/chrome.exe',
'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
].filter(Boolean).find(existsSync);
browser=await chromium.launch({headless:true,...(executablePath?{executablePath}:{})});
for(const width of [1280,640]){
const page=await browser.newPage({viewport:{width,height:900}}),errors=[];page.on('pageerror',e=>errors.push(String(e)));
await page.route('**/orientation-proof',r=>r.fulfill({contentType:'text/html',body:'<html><body><script type="module" src="/'+name+'"></script></body></html>'}));
await page.goto(origin+'/orientation-proof');
const panel=page.locator('[data-mapping-installation-orientation]');await panel.waitFor();
const match=panel.getByRole('button',{name:'基準灯体と同じ姿勢に揃える'}),aim=panel.getByRole('button',{name:'Aim installation axis (+Z) at target'});
await match.click();assert.equal(await aim.isDisabled(),true);await page.evaluate(()=>window.proof.finish());await page.waitForFunction(()=>!document.querySelector('[data-mapping-installation-orientation] button').disabled);
await panel.getByLabel('Target X').fill('-2.5');await panel.getByLabel('Target Y').fill('3');await panel.getByLabel('Target Z').fill('4');
await aim.focus();await aim.press('Enter');assert.deepEqual(await page.evaluate(()=>window.proof.calls),['match',{x:-2.5,y:3,z:4}]);
await panel.getByLabel('Target X').fill('');assert.equal(await aim.isDisabled(),true);await panel.getByLabel('Target X').fill('0');
await page.evaluate(()=>window.proof.setCount(1));assert.equal(await match.isDisabled(),true);assert.equal(await aim.isEnabled(),true);
const size=await panel.evaluate(el=>{const r=el.getBoundingClientRect();return {left:r.left,right:r.right,width:r.width,scroll:el.scrollWidth,client:el.clientWidth,buttons:[...el.querySelectorAll('button')].map(b=>b.getBoundingClientRect().height),buttonWidths:[...el.querySelectorAll('button')].map(b=>b.getBoundingClientRect().width)}});
assert(size.left>=0&&size.right<=width&&size.scroll<=size.client+1,JSON.stringify(size));assert(size.buttons.every(h=>h>=28),JSON.stringify(size));
assert(size.buttonWidths.every(w=>w>=size.client-18), 'Long operation labels get a full row rather than narrow grid columns');
await page.screenshot({path:resolve(artifacts,'orientation-'+width+'.png')});
await page.evaluate(()=>window.proof.setCount(0));assert.equal(await panel.count(),0);assert.deepEqual(errors,[]);await page.close();
}
console.log('PASS mapping orientation UI at1280/640: axis inputs, keyboard action, busy exclusion, selection availability and unchanged control sizing/containment.');
}finally{await browser?.close();vite.kill();await unlink(fixture).catch(()=>{});}
