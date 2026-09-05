import assert from "node:assert/strict";
import {readFile} from "node:fs/promises";
import {createRequire} from "node:module";
import {pathToFileURL} from "node:url";
import ts from "typescript";
const require=createRequire(import.meta.url),solidUrl=pathToFileURL(require.resolve('solid-js/dist/solid.js')).href;
const {createRoot}=await import(solidUrl),compiled=new Map();
async function compile(url){
  if(compiled.has(url.href))return compiled.get(url.href);
  let code=ts.transpileModule(await readFile(url,'utf8'),{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ESNext}}).outputText.replaceAll('from "solid-js"',`from "${solidUrl}"`);
  for(const match of [...code.matchAll(/from\s+"(\.[^"]+)"/g)])code=code.replace(match[0],`from "${await compile(new URL(match[1]+'.ts',url))}"`);
  const result='data:text/javascript;base64,'+Buffer.from(code).toString('base64');compiled.set(url.href,result);return result;
}
const {mappingInstallationRotationToward:aim,rotateMappingFixtureDirection:rotate}=await import(await compile(new URL('../src/mappingFixtureOrientation.ts',import.meta.url)));
const {createMappingLayoutController}=await import(await compile(new URL('../src/createMappingLayoutController.ts',import.meta.url)));
const near=(actual,expected)=>assert.ok(Math.abs(actual-expected)<1e-10,`${actual} != ${expected}`);
const origin={x:0,y:0,z:0};
for(const target of [{x:0,y:0,z:1},{x:1,y:0,z:0},{x:0,y:0,z:-1},{x:0,y:1,z:0},{x:0,y:-1,z:0},{x:-3,y:4,z:-5}]){
  const angle=aim(origin,target),direction=rotate({x:0,y:0,z:1},angle),length=Math.hypot(target.x,target.y,target.z);
  for(const axis of ['x','y','z'])near(direction[axis],target[axis]/length);
  assert.equal(angle.roll,0);
}
const tilted=rotate({x:1,y:0,z:0},{yaw:0,pitch:0,roll:90});near(tilted.x,0);near(tilted.y,1);
assert.throws(()=>aim(origin,origin),/同じ位置/);
assert.throws(()=>aim(origin,{x:NaN,y:0,z:1}),/有限/);
assert.throws(()=>aim({x:-Number.MAX_VALUE,y:0,z:0},{x:Number.MAX_VALUE,y:0,z:0}),/同じ位置/);
function fixture(){
  let fixtures=[{id:1,label:'A',position:{x:0,y:0,z:0},rotation:{yaw:30,pitch:40,roll:50}},
    {id:2,label:'B',position:{x:4,y:1,z:-3},rotation:{yaw:60,pitch:70,roll:80}}];
  let active=fixtures[1],selected=fixtures,rejectId=null,gate=null,refreshCount=0;
  const calls=[],messages=[];let controller,dispose;
  createRoot(cleanup=>{dispose=cleanup;controller=createMappingLayoutController({
    snapshot:()=>({fixtures,stage_objects:[]}),filteredFixtures:()=>fixtures,selectedFixtureGroupFilter:()=>null,
    selectedMappingFixtures:()=>selected,selectedStageObject:()=>null,selectedFixture:()=>active,
    selectedMappingFixtureIds:()=>selected.map(f=>f.id),setSelectedMappingFixtureIds:()=>{},activateFixture:()=>{},
    setFixtureTransform:async(f,update,refresh)=>{calls.push({id:f.id,update,refresh});if(gate)await gate;if(f.id===rejectId)return false;fixtures=fixtures.map(old=>old.id===f.id?{...old,...update}:old);return true;},
    snapStagePosition:point=>point,mappingWorldToStageObjectLocal:point=>point,
    refreshSnapshot:async()=>{refreshCount++;return {fixtures};},setMessage:value=>messages.push(value),
  });});
  return {controller,dispose,calls,messages,get fixtures(){return fixtures;},get refreshCount(){return refreshCount;},set selected(v){selected=v;},set active(v){active=v;},set rejectId(v){rejectId=v;},set gate(v){gate=v;}};
}
{
  const f=fixture(),positions=f.fixtures.map(v=>v.position);await f.controller.matchSelectedMappingFixtureOrientations();
  assert.deepEqual(f.calls.map(c=>c.update),[{rotation:{yaw:60,pitch:70,roll:80}},{rotation:{yaw:60,pitch:70,roll:80}}]);
  assert(f.calls.every(c=>c.refresh===false));assert.equal(f.refreshCount,1);
  assert.deepEqual(f.fixtures.map(v=>v.position),positions);f.dispose();
}
{
  const f=fixture();f.active={id:999,rotation:{yaw:999}};await f.controller.matchSelectedMappingFixtureOrientations();
  assert.deepEqual(f.calls[0].update.rotation,{yaw:30,pitch:40,roll:50});f.dispose();
}
{
  const f=fixture(),target={x:1,y:2,z:3};await f.controller.aimSelectedMappingFixtureInstallationAxes(target);
  for(const fixture of f.fixtures){const direction=rotate({x:0,y:0,z:1},fixture.rotation);const delta={x:target.x-fixture.position.x,y:target.y-fixture.position.y,z:target.z-fixture.position.z};const length=Math.hypot(delta.x,delta.y,delta.z);for(const axis of ['x','y','z'])near(direction[axis],delta[axis]/length);}
  assert.equal(f.refreshCount,1);f.dispose();
}
{
  const f=fixture();await f.controller.aimSelectedMappingFixtureInstallationAxes(f.fixtures[1].position);
  assert.equal(f.calls.length,0,'Invalid later fixture rejects the entire plan before first mutation');assert.equal(f.refreshCount,0);f.dispose();
}
{
  const f=fixture();f.rejectId=1;await f.controller.matchSelectedMappingFixtureOrientations();
  assert.equal(f.calls.length,1);assert.equal(f.refreshCount,1);assert.match(f.messages.at(-1),/earlier fixtures may already have changed/);f.dispose();
}
{
  const f=fixture();let release;f.gate=new Promise(r=>release=r);
  const pending=f.controller.matchSelectedMappingFixtureOrientations();await f.controller.aimSelectedMappingFixtureInstallationAxes({x:1,y:2,z:3});assert.equal(f.calls.length,1);
  release();await pending;assert.equal(f.calls.length,2);f.dispose();
}
{
  const f=fixture();f.selected=[];await f.controller.matchSelectedMappingFixtureOrientations();assert.equal(f.calls.length,0);assert.match(f.messages[0],/選択/);f.dispose();
}
console.log('PASS mounting-axis orientation: cardinal/vertical/oblique directions, full validation, selected anchor, rotation-only batch, failure report and concurrent exclusion.');
