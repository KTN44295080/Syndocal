import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { mkdir, writeFile, unlink } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { homedir } from 'node:os';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire(import.meta.url);
let chromium;
try { ({ chromium } = require('playwright')); }
catch { ({ chromium } = require(resolve(homedir(), '.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright'))); }
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const name = `.group-rotation-proof-${process.pid}.tsx`, fixture = resolve(root, name);
const artifacts = resolve(root, '../target/qa/mapping-group-rotation-20260906');
await mkdir(artifacts, { recursive: true });
await writeFile(fixture, `
import {createSignal,For} from 'solid-js';import {render} from 'solid-js/web';
import {createMappingRenderModel} from '/src/createMappingRenderModel';
import {MappingBeamsLayer} from '/src/components/MappingBeamsLayer';
const identity=[1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1];
const make=(id,yaw)=>({id,label:'Fixture '+id,manufacturer:'Test',profile_name:'Spot',mode_name:'Static',universe:1,address:id,
 position:{x:(id-2)*50,y:0,z:0},rotation:{yaw,pitch:0,roll:0},group_ids:[],controls:[{attribute:'Dimmer',channel_name:'Dimmer',offsets:[id],resolution:'U8',default_value:65535,functions:[]}],
 attribute_values:[{attribute:'Dimmer',value:65535}],geometries:[{name:'Beam',kind:'Beam',matrix:identity,beam_angle_deg:10}],highlighted:false,soloed:false,parked:false});
const [fixtures,setFixtures]=createSignal([make(1,20),make(2,350),make(3,120)]);
const [drag,setDrag]=createSignal(null);
render(()=>{const model=createMappingRenderModel({mappingDrag:drag,mappingShowGeometry:()=>false,mappingFilteredFixtures:fixtures,liveFixtures:fixtures,
 mappingViewportBox:()=>({x:0,z:0,width:100,height:100}),dmxPreviews:()=>[],stageWorldBounds:()=>({minX:-100,maxX:100,minZ:-100,maxZ:100}),
 selectedMappingFixtureIdSet:()=>new Set([1,2]),selectedFixtureGroupFilter:()=>null,selectedFixtureId:()=>1,faderValues:()=>({}),snapshot:()=>({video:{outputs:[]},stage_objects:[]}),
 selectedStageObjectId:()=>null,snapStagePoint:p=>p,snapStagePosition:p=>p,snapStageLength:n=>n});
 window.proof={
  start:()=>setDrag({kind:'fixtureYaw',pointerId:1,projectEpoch:1,fixtureId:1,fixtureIds:[1,2],
   startRotations:Object.fromEntries(fixtures().filter(f=>f.id!==3).map(f=>[f.id,{...f.rotation}])),
   startWorld:{x:-50,z:-1},currentWorld:{x:-49,z:0},centerWorld:{x:-50,z:0},startClient:{x:0,y:0},currentClient:{x:20,y:20}}),
  cancel:()=>setDrag(null),
  confirm:()=>{setFixtures(fixtures().map(f=>({...f,rotation:{...f.rotation,yaw:model.mappingFixtureYaw(f)}})));setDrag(null);},
  snapshot:()=>fixtures().map(f=>({id:f.id,position:f.position,rotation:f.rotation,previewYaw:model.mappingFixtureYaw(f)}))
 };
 return <main style={{background:'#0b131b',padding:'24px',color:'white'}}><h2>Selected fixtures 1 and 2 — rotation preview</h2>
 <svg viewBox='0 0 100 100' style={{width:'100%',height:'420px'}}><MappingBeamsLayer fixtures={model.mappingStageFixtures()}/>
 <For each={fixtures()}>{f=><text data-fixture={f.id} data-yaw={model.mappingFixtureYaw(f)} x={25*f.id} y='55' fill='white' font-size='3'>{f.label}: {model.mappingFixtureYaw(f)}°</text>}</For></svg></main>;
},document.body);
`);
const port = 5203, origin = `http://127.0.0.1:${port}`;
const vite = spawn(process.execPath, [resolve(root, 'node_modules/vite/bin/vite.js'), '--host', '127.0.0.1', '--port', String(port), '--strictPort'], { cwd: root, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] });
let log = '', browser;
vite.stdout.on('data', data => log += data); vite.stderr.on('data', data => log += data);
try {
  for (let i = 0; ; i++) {
    assert.equal(vite.exitCode, null, log);
    try { if ((await fetch(origin)).ok) break; } catch {}
    assert(i < 150, log); await new Promise(resolve => setTimeout(resolve, 100));
  }
  const executablePath = [
    process.env.CHROME_PATH,
    process.env.EDGE_PATH,
    process.env.LOCALAPPDATA ? resolve(process.env.LOCALAPPDATA, 'Google/Chrome/Application/chrome.exe') : null,
    process.env.LOCALAPPDATA ? resolve(process.env.LOCALAPPDATA, 'Microsoft/Edge/Application/msedge.exe') : null,
    'C:/Program Files/Google/Chrome/Application/chrome.exe',
    'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
  ].filter(Boolean).find(existsSync);
  browser = await chromium.launch({ headless: true, ...(executablePath ? { executablePath } : {}) });
  const page = await browser.newPage({ viewport: { width: 1000, height: 620 } });
  const errors = []; page.on('pageerror', error => errors.push(String(error)));
  await page.route('**/group-rotation-proof', route => route.fulfill({ contentType: 'text/html', body: `<html><body><script type="module" src="/${name}"></script></body></html>` }));
  await page.goto(origin + '/group-rotation-proof');
  await page.locator('[data-fixture="1"]').waitFor();
  const points = () => page.locator('[data-stage-beam-fixture-id]').evaluateAll(elements => elements.map(element => element.getAttribute('points')));
  const before = await points(); assert.equal(before.length, 3);
  await page.evaluate(() => window.proof.start());
  await page.waitForFunction(() => document.querySelector('[data-fixture="2"]').getAttribute('data-yaw') === '60');
  const preview = await points();
  assert.notEqual(preview[0], before[0]); assert.notEqual(preview[1], before[1]); assert.equal(preview[2], before[2]);
  assert.deepEqual(await page.evaluate(() => window.proof.snapshot().map(f => f.previewYaw)), [90, 60, 120]);
  await page.screenshot({ path: resolve(artifacts, 'selected-preview.png') });
  await page.evaluate(() => window.proof.cancel());
  assert.deepEqual(await points(), before);
  await page.evaluate(() => { window.proof.start(); window.proof.confirm(); });
  assert.deepEqual(await points(), preview);
  const snapshot = await page.evaluate(() => window.proof.snapshot());
  assert.deepEqual(snapshot.map(f => f.rotation.yaw), [90, 60, 120]);
  assert.deepEqual(snapshot.map(f => f.position), [{ x: -50, y: 0, z: 0 }, { x: 0, y: 0, z: 0 }, { x: 50, y: 0, z: 0 }]);
  assert.deepEqual(errors, []);
  await page.screenshot({ path: resolve(artifacts, 'confirmed.png') });
  console.log('PASS real mapping render model/beam layer: both selected previews rotate, relative yaw and positions preserved, unselected unchanged, cancel/confirmed snapshot correct. Controller persistence is verified separately.');
} finally {
  await browser?.close(); vite.kill(); await unlink(fixture).catch(() => {});
}
