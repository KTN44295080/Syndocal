import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { dirname,resolve } from 'node:path';
import { homedir } from 'node:os';
import { fileURLToPath } from 'node:url';
import { mkdir,writeFile,unlink } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { createServer } from 'vite';
import { createServer as createTcpServer } from 'node:net';
const require=createRequire(import.meta.url);let chromium;
try{({chromium}=require('playwright'));}catch{({chromium}=require(resolve(homedir(),'.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright')));}
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..'),name='.scene-rename-proof-'+process.pid+'.tsx',fixture=resolve(root,name);
const artifacts=resolve(process.env.SYNDOCAL_SCENE_RENAME_ARTIFACT_DIR??resolve(root,'../target/qa/scene-rename-alpha77'));await mkdir(artifacts,{recursive:true});let server,browser;
try{
await writeFile(fixture,`
import {createSignal} from 'solid-js';import {render} from 'solid-js/web';import {SceneMatrixPanel} from '/src/components/SceneMatrixPanel';import '/src/styles.css';
const initial={id:1,label:'Old scene',cue_number:'1',cue_list_id:1,fade_ms:0,targets:[],effect_targets:[],recall_mode:'Coexist'};
const [cues,setCues]=createSignal([{...initial}]),[selected,setSelected]=createSignal(1),[epoch,setEpoch]=createSignal(1);let reject=false,wait=null;
window.proof={loadSameId:()=>{setEpoch(2);setCues([{...initial,label:'Other project'}])},calls:[],triggers:[],releases:[],flash:()=>setCues(cues().map(c=>({...c,live_modifier_defaults:{speed:1,size:1,phase:0,direction:'Forward',segment:0,flash:true}}))),reject:v=>reject=v,poll:()=>setCues(cues().map(c=>({...c}))),replace:()=>{setCues([]);queueMicrotask(()=>setCues([{...initial,label:'Replacement'}]))},delay:()=>{let resolve; wait=new Promise(r=>resolve=r);return ()=>resolve()},};
const noop=()=>{},props={get projectEpoch(){return epoch()},get bankAuthority(){return {issue:null,cues:cues(),cueLists:[{id:1,label:'Bank 1',active_cue_id:null}]}},selectedCueListId:1,onSelectCueList:noop,onCueListLabel:noop,onCreateCueList:noop,onRenameCueList:noop,groupIds:[],activeCueId:null,activeGroupCueIds:{},get selectedCueId(){return selected()},onSelectCue:setSelected,onTriggerCue:id=>window.proof.triggers.push(id),onReleaseCue:id=>window.proof.releases.push(id),onOpenSuperScene:noop,onCreateCue:noop,onAddTimelineCue:noop,onMoveCue:noop,onBeginTimelineCueDrag:noop,onMoveTimelineCueDrag:noop,onEndTimelineCueDrag:noop,timelineTrack:'Lighting',onRenameCue:async(id,label)=>{window.proof.calls.push({id,label});if(wait){let p=wait;wait=null;await p;}if(reject)return false;setCues(cues().map(c=>c.id===id?{...c,label}:c));return true;}};
render(()=><div style='height:100vh;padding:16px;box-sizing:border-box'><style>{'.sceneMatrixPanel{display:flex;flex-direction:column;height:700px}.sceneMatrixScroller{flex:1;min-height:0}'}</style><SceneMatrixPanel {...props}/><button id='outside'>Outside</button></div>,document.body);
`);
const reservation=createTcpServer();await new Promise((resolve,reject)=>{reservation.once('error',reject);reservation.listen(0,'127.0.0.1',resolve);});const reservedPort=reservation.address().port;await new Promise((resolve,reject)=>reservation.close(e=>e?reject(e):resolve()));
server=await createServer({root,server:{host:'127.0.0.1',port:reservedPort,strictPort:true},logLevel:'error',plugins:[{name:'scene-rename-proof',configureServer(s){s.middlewares.use((req,res,next)=>{if(!req.url?.startsWith('/scene-rename-proof'))return next();res.setHeader('Content-Type','text/html');res.end('<html><head><link rel="icon" href="data:,"></head><body><script type="module" src="/'+name+'"></script></body></html>');});}}]});await server.listen();
const port=server.httpServer.address().port,executablePath=['C:/Program Files/Google/Chrome/Application/chrome.exe','C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe'].find(existsSync);browser=await chromium.launch({headless:true,...(executablePath?{executablePath}:{})});const page=await browser.newPage(),errors=[],results=[];page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error')errors.push(m.text())});
for(const width of [1280,1920]) {
 await page.setViewportSize({width,height:900}); await page.goto('http://127.0.0.1:'+port+'/scene-rename-proof');
 const name=page.locator('[data-scene-matrix-cue-name]'),input=page.locator('[data-scene-matrix-rename-input]');await name.waitFor();
 await name.dblclick();await input.waitFor();await input.fill('Japanese scene 人生');await page.evaluate(()=>window.proof.poll());assert.equal(await input.inputValue(),'Japanese scene 人生');await input.press('Enter');await input.waitFor({state:'detached'});assert.equal(await name.textContent(),'Japanese scene 人生');assert.deepEqual(await page.evaluate(()=>window.proof.triggers),[]);
 await name.dblclick();await input.fill('Cancelled');await input.press('Escape');assert.equal(await name.textContent(),'Japanese scene 人生');assert.equal(await page.evaluate(()=>window.proof.calls.length),1);
 await page.locator('[data-scene-matrix-edit-strip]').focus();await page.keyboard.press('F2');await input.fill('Blur saved');await page.locator('#outside').click();await input.waitFor({state:'detached'});assert.equal(await name.textContent(),'Blur saved');
 await page.evaluate(()=>window.proof.reject(true));await name.dblclick();await input.fill('Rejected');await input.press('Enter');await page.getByRole('alert').waitFor();assert.equal(await input.inputValue(),'Rejected');assert.equal(await name.textContent(),'Blur saved');await page.evaluate(()=>window.proof.reject(false));await input.press('Enter');await input.waitFor({state:'detached'});assert.equal(await name.textContent(),'Rejected');
 await name.dblclick();await input.fill('');await input.press('Enter');await page.getByRole('alert').waitFor();assert.equal(await page.evaluate(()=>window.proof.calls.length),4);await input.press('Escape');
 await page.evaluate(()=>window.proof.flash());await name.dblclick();await input.fill('Flash name');await input.press('Enter');await input.waitFor({state:'detached'});assert.deepEqual(await page.evaluate(()=>window.proof.triggers),[]);assert.deepEqual(await page.evaluate(()=>window.proof.releases),[]);
 await page.evaluate(()=>{window.release=window.proof.delay()});await name.dblclick();await input.fill('Old pending');await input.press('Enter');await page.evaluate(()=>window.proof.replace());await input.waitFor({state:'detached'});await name.dblclick();assert.equal(await input.inputValue(),'Replacement');await page.evaluate(()=>window.release());assert.equal(await input.isVisible(),true,'Late save must not close a replacement editor');await input.press('Escape');
 await name.dblclick();await input.fill('Old project draft');await page.evaluate(()=>window.proof.loadSameId());await input.waitFor({state:'detached'});assert.equal(await name.textContent(),'Other project');
 assert.deepEqual(errors,[]);await page.screenshot({path:resolve(artifacts,'scene-rename-'+width+'.png')});results.push({width,enter:true,blur:true,escape:true,pollRetainsDraft:true,rejectionRetainsDraft:true,emptyRejected:true,noSceneTrigger:true,noFlashPressOrRelease:true,replacementEditorPreserved:true,sameIdNewProjectRetiresDraft:true});
}
await writeFile(resolve(artifacts,'report.json'),JSON.stringify({passed:true,results,errors,nativeWindow:false},null,2));console.log('PASS: inline scene rename at 1280/1920; '+artifacts);
}finally{await browser?.close();await server?.close();await unlink(fixture).catch(()=>{});}
