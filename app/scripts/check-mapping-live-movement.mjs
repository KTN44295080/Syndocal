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
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..'), name=`.live-movement-proof-${process.pid}.tsx`, fixture=resolve(root,name);
const artifacts=resolve(root,'../target/qa/mapping-live-movement-20260905'); await mkdir(artifacts,{recursive:true});
await writeFile(fixture,`
import {createSignal,For} from 'solid-js';import {render} from 'solid-js/web';
import {createMappingRenderModel} from '/src/createMappingRenderModel';
import {mappingFixtureLiveMovement} from '/src/mappingFixtureBeam';
import {MappingBeamsLayer} from '/src/components/MappingBeamsLayer';
const identity=[1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1];
const [pitch,setPitch]=createSignal(0);const [pan,setPan]=createSignal(32768);const [tilt,setTilt]=createSignal(32768);const [preview,setPreview]=createSignal([]);window.proof={setPitch,setPan,setTilt,setPreview};
const fixture=(id,angle,yaw)=>({id,label:'Beam '+angle,manufacturer:'Test',profile_name:'Spot',mode_name:'Static',universe:1,address:id,
 position:{x:(id-2)*65,y:0,z:0},rotation:{yaw,pitch:id===2?pitch():0,roll:0},group_ids:[],controls:[{attribute:'Dimmer',channel_name:'Dimmer',offsets:[id],resolution:'U8',default_value:65535,functions:[]},{attribute:'Pan',channel_name:'Pan',offsets:[5,6],resolution:'SixteenBit',default_value:32768},{attribute:'Tilt',channel_name:'Tilt',offsets:[7],resolution:'EightBit',default_value:32768},...(id===3?[{attribute:'Red1',channel_name:'Red1',offsets:[8],resolution:'EightBit',default_value:65535},{attribute:'Red2',channel_name:'Red2',offsets:[9],resolution:'EightBit',default_value:65535}]:[])],
 attribute_values:[{attribute:'Dimmer',value:65535},{attribute:'Pan',value:0},{attribute:'Tilt',value:0}],geometries:[{name:'Beam',kind:'Beam',matrix:identity,beam_angle_deg:angle}],highlighted:false,soloed:false,parked:false});
const fixtures=()=>[fixture(1,6,30),fixture(2,10,0),fixture(3,30,-30)];
window.proof.readMovement=()=>mappingFixtureLiveMovement(fixtures()[1],new Map(preview().map(p=>[p.universe,p.values])));
render(()=>{const model=createMappingRenderModel({mappingDrag:()=>null,mappingShowGeometry:()=>false,mappingFilteredFixtures:fixtures,liveFixtures:()=>fixtures().map(f=>({...f,attribute_values:[{attribute:'Dimmer',value:65535},{attribute:'Pan',value:pan()},{attribute:'Tilt',value:tilt()}]})),
 mappingViewportBox:()=>({x:-200,z:-200,width:400,height:400}),dmxPreviews:preview,stageWorldBounds:()=>({minX:-100,maxX:100,minZ:-60,maxZ:60}),
 selectedMappingFixtureIdSet:()=>new Set(),selectedFixtureGroupFilter:()=>null,selectedFixtureId:()=>null,faderValues:()=>({'1:Pan':0,'2:Pan':0,'3:Pan':0}),snapshot:()=>({video:{outputs:[]},stage_objects:[]}),
 selectedStageObjectId:()=>null,snapStagePoint:p=>p,snapStagePosition:p=>p,snapStageLength:n=>n});
 return <main style={{background:'#0b131b',padding:'24px',color:'white'}}><h2>2D beam projection</h2><svg viewBox='-10 0 120 120' style={{width:'100%',height:'500px'}}>
 <MappingBeamsLayer fixtures={model.mappingStageFixtures()}/><For each={model.mappingStageFixtures()}>{f=><><circle cx={f.x} cy={f.z} r='1' fill='white'/><text x={f.x} y={f.z-4} fill='white' font-size='3' text-anchor='middle'>{f.label}</text></>}</For></svg></main>;
},document.body);`);
const port=5203,origin=`http://127.0.0.1:${port}`;
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
 await page.goto(origin+'/beam-proof');const beams=page.locator('polygon.stageBeam');await beams.first().waitFor();assert.equal(await beams.count(),4);
 assert.match(await beams.nth(0).textContent(),/6°/);assert.match(await beams.nth(2).textContent(),/30°/);
 const beam=page.locator('[data-stage-beam-fixture-id="2"]');
 const segments=page.locator('[data-stage-beam-fixture-id="3"]');
 assert.equal(await segments.count(),2);
 const originalSegments=await segments.evaluateAll(nodes=>nodes.map(n=>n.getAttribute('points')));
 const initial=await beam.getAttribute('points');
 await page.evaluate(()=>window.proof.setPan(44000));
 await page.waitForFunction(previous=>document.querySelector('[data-stage-beam-fixture-id="2"]').getAttribute('points')!==previous,initial);
 const moved=await beam.getAttribute('points');
 const movedSegments=await segments.evaluateAll(nodes=>nodes.map(n=>n.getAttribute('points')));
 for(let i=0;i<2;i++)assert.notEqual(movedSegments[i],originalSegments[i],'each segment beam follows live Pan');
 await page.evaluate(()=>window.proof.setTilt(45000));
 await page.waitForFunction(previous=>document.querySelector('[data-stage-beam-fixture-id="2"]').getAttribute('points')!==previous,moved);
 const tilted=await beam.getAttribute('points');
 await page.evaluate(()=>window.proof.setPreview([{universe:1,values:Array.from({length:512},(_,i)=>i===5?128:i===6?1:i===7?128:255)}]));
 await page.waitForFunction(previous=>document.querySelector('[data-stage-beam-fixture-id="2"]').getAttribute('points')!==previous,tilted);
 assert.deepEqual(await page.evaluate(()=>window.proof.readMovement()),{pan:32769,tilt:32896});
 await page.evaluate(()=>window.proof.setPreview([{universe:1,values:Array.from({length:512},(_,i)=>i===5?128:i===6?2:i===7?128:255)}]));
 await page.waitForFunction(()=>window.proof.readMovement().pan===32770);
 const dmx=await beam.getAttribute('points');
 await page.evaluate(()=>{window.proof.setPan(1000);window.proof.setTilt(1000);});
 await page.waitForTimeout(60);assert.equal(await beam.getAttribute('points'),dmx,'DMX preview must override attributes and programmer values');
 await page.evaluate(()=>window.proof.setPreview([]));
 await page.waitForFunction(previous=>document.querySelector('[data-stage-beam-fixture-id="2"]').getAttribute('points')!==previous,dmx);
 await page.screenshot({path:resolve(artifacts,'live-pan-tilt.png')});assert.deepEqual(errors,[]);

 console.log('PASS live Pan/Tilt actual Mapping model and beam layer: attribute motion, stale programmer isolation, 16-bit DMX precedence, cache invalidation and preview removal fallback.');
} finally {await browser?.close();vite.kill();await unlink(fixture).catch(()=>{});}
