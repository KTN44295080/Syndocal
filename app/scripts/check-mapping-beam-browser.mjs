import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { mkdir, writeFile, unlink } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { homedir } from 'node:os';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
const require = createRequire(import.meta.url);
let chromium; try { ({chromium}=require('playwright')); } catch { ({chromium}=require(resolve(homedir(),'.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright'))); }
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..'), name=`.beam-proof-${process.pid}.tsx`, fixture=resolve(root,name);
const artifacts=resolve(root,'../target/qa/mapping-beam-20260905'); await mkdir(artifacts,{recursive:true});
await writeFile(fixture,`
import {createSignal,For} from 'solid-js';import {render} from 'solid-js/web';
import {createMappingRenderModel} from '/src/createMappingRenderModel';
import {MappingBeamsLayer} from '/src/components/MappingBeamsLayer';
const identity=[1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1];
const [pitch,setPitch]=createSignal(0);window.proof={setPitch};
const fixture=(id,angle,yaw)=>({id,label:'Beam '+angle,manufacturer:'Test',profile_name:'Spot',mode_name:'Static',universe:1,address:id,
 position:{x:(id-2)*65,y:0,z:0},rotation:{yaw,pitch:id===2?pitch():0,roll:0},group_ids:[],controls:[{attribute:'Dimmer',channel_name:'Dimmer',offsets:[id],resolution:'U8',default_value:65535,functions:[]}],
 attribute_values:[{attribute:'Dimmer',value:65535}],geometries:[{name:'Beam',kind:'Beam',matrix:identity,beam_angle_deg:angle}],highlighted:false,soloed:false,parked:false});
const fixtures=()=>[fixture(1,6,30),fixture(2,10,0),fixture(3,30,-30)];
render(()=>{const model=createMappingRenderModel({mappingDrag:()=>null,mappingShowGeometry:()=>false,mappingFilteredFixtures:fixtures,liveFixtures:fixtures,
 mappingViewportBox:()=>({x:-200,z:-200,width:400,height:400}),dmxPreviews:()=>[],stageWorldBounds:()=>({minX:-100,maxX:100,minZ:-60,maxZ:60}),
 selectedMappingFixtureIdSet:()=>new Set(),selectedFixtureGroupFilter:()=>null,selectedFixtureId:()=>null,faderValues:()=>({}),snapshot:()=>({video:{outputs:[]},stage_objects:[]}),
 selectedStageObjectId:()=>null,snapStagePoint:p=>p,snapStagePosition:p=>p,snapStageLength:n=>n});
 return <main style={{background:'#0b131b',padding:'24px',color:'white'}}><h2>2D beam projection</h2><svg viewBox='-10 0 120 120' style={{width:'100%',height:'500px'}}>
 <MappingBeamsLayer fixtures={model.mappingStageFixtures()}/><For each={model.mappingStageFixtures()}>{f=><><circle cx={f.x} cy={f.z} r='1' fill='white'/><text x={f.x} y={f.z-4} fill='white' font-size='3' text-anchor='middle'>{f.label}</text></>}</For></svg></main>;
},document.body);`);
const port=5201,origin=`http://127.0.0.1:${port}`;
const vite=spawn(process.execPath,[resolve(root,'node_modules/vite/bin/vite.js'),'--host','127.0.0.1','--port',String(port),'--strictPort'],{cwd:root,windowsHide:true,stdio:['ignore','pipe','pipe']});
let log='',browser;vite.stdout.on('data',d=>log+=d);vite.stderr.on('data',d=>log+=d);
try {
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
 const page=await browser.newPage({viewport:{width:1000,height:650}}),errors=[];page.on('pageerror',e=>{errors.push(String(e));console.error(String(e));});
 await page.route('**/beam-proof',r=>r.fulfill({contentType:'text/html',body:'<html><body><script type="module" src="/'+name+'"></script></body></html>'}));
 await page.goto(origin+'/beam-proof');const beams=page.locator('polygon.stageBeam');await beams.first().waitFor();assert.equal(await beams.count(),3);
 assert.match(await beams.nth(0).textContent(),/6°/);assert.match(await beams.nth(2).textContent(),/30°/);
 const beam=page.locator('[data-stage-beam-fixture-id="2"]');const before=await beam.getAttribute('points');
 await page.screenshot({path:resolve(artifacts,'beam-angles.png')});
 await page.evaluate(()=>window.proof.setPitch(90));await page.waitForFunction(previous=>document.querySelector('[data-stage-beam-fixture-id="2"]').getAttribute('points')!==previous,before);
 const bounds=await beam.evaluate(el=>{const b=el.getBBox();return {width:b.width,height:b.height};});assert(Math.abs(bounds.width-bounds.height)<.01,'Vertical projection must be a footprint');
 await page.screenshot({path:resolve(artifacts,'beam-vertical.png')});assert.deepEqual(errors,[]);
 console.log('PASS actual Mapping render model and beam layer: profile optics, three emitted beams, reactive mounting tilt and vertical footprint.');
} finally {await browser?.close();vite.kill();await unlink(fixture).catch(()=>{});}
