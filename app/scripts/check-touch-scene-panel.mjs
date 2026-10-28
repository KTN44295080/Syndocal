import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import ts from 'typescript';

// Pure policy behind the Control > Lighting Scene panel and the header tempo placement.
const require=createRequire(import.meta.url),cache=new Map();
const root=resolve(dirname(fileURLToPath(import.meta.url)),'../src');
function load(file){if(cache.has(file))return cache.get(file);const exports={};cache.set(file,exports);
  const js=ts.transpileModule(readFileSync(file,'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText;
  new Function('require','exports',js)(name=>{if(!name.startsWith('.'))return require(name);const target=resolve(dirname(file),name.endsWith('.ts')?name:name+'.ts');assert.ok(existsSync(target),target);return load(target);},exports);return exports;}
const scenes=load(resolve(root,'touchScenePanel.ts'));
const {defaultTouchSurface}=load(resolve(root,'touchSurface.ts'));
const {inspectBankAuthority}=load(resolve(root,'bankAuthority.ts'));
const {nextTopbarTempoPlacement}=load(resolve(root,'topbarTempoPlacement.ts'));

const cue=(id,cue_list_id,extra={})=>({id,cue_list_id,cue_number:String(id),label:`Scene ${id}`,fade_ms:0,pre_wait_ms:0,...extra});
const banks=[{id:2,label:'Bank B'},{id:1,label:'Bank A'},{id:3,label:'Empty'}];
const cues=[cue(1,1),cue(2,1),cue(3,2),cue(4,1),cue(5,2),cue(6,2),cue(7,1),cue(8,2),cue(9,2)];
const authority=inspectBankAuthority(banks,cues,[]);
assert.equal(authority.issue,null);
const grouped=scenes.touchSceneBanks(authority);
assert.deepEqual(grouped.map(entry=>entry.bank.label),['Bank B','Bank A'],'Banks keep authored order; empty Banks are omitted');
assert.deepEqual(grouped.map(entry=>entry.cues.map(c=>c.id)),[[3,5,6,8,9],[1,2,4,7]],'every Scene appears once, in its Bank, in authored order');
assert.equal(grouped.flatMap(entry=>entry.cues).length,cues.length,'all nine Scenes are reachable (not only the first four)');
const broken=inspectBankAuthority(banks,[...cues,cue(10,99)],[]);
assert.notEqual(broken.issue,null);
assert.deepEqual(scenes.touchSceneBanks(broken),[],'an unavailable authority never yields a valid-looking subset');

assert.equal(scenes.touchSceneActive(cue(4,1),{active_cue_id:4}),true);
assert.equal(scenes.touchSceneActive(cue(4,1),{active_cue_id:5}),false);
assert.equal(scenes.touchSceneActive(cue(4,1,{group_id:'front'}),{active_cue_id:4,active_group_cue_ids:{front:7}}),false,'grouped Scenes follow their group activity');
assert.equal(scenes.touchSceneActive(cue(7,1,{group_id:'front'}),{active_cue_id:null,active_group_cue_ids:{front:7}}),true);

const desk=defaultTouchSurface(cues).pages[0];
assert.equal(scenes.touchScenePanelColumnsFree(desk),true,'the Default Desk leaves the last three columns free');
const full={...desk,controls:[...desk.controls,{id:99,kind:'Button',x:10,y:0,w:2,h:1,label:'X',binding:null}]};
assert.equal(scenes.touchScenePanelColumnsFree(full),false);
assert.equal(scenes.touchScenePanelOpen(null,desk),true,'auto: open over free columns');
assert.equal(scenes.touchScenePanelOpen(null,full),false,'auto: never cover authored controls');
assert.equal(scenes.touchScenePanelOpen('open',full),true,'explicit choice wins');
assert.equal(scenes.touchScenePanelOpen('closed',desk),false,'explicit choice wins');

const storage=new Map();
globalThis.localStorage={getItem:key=>storage.get(key)??null,setItem:(key,value)=>storage.set(key,value)};
assert.equal(scenes.readTouchScenePanelPreference(),null);
scenes.writeTouchScenePanelPreference('closed');
assert.equal(scenes.readTouchScenePanelPreference(),'closed');
storage.set(scenes.touchScenePanelStorageKey,'future');
assert.equal(scenes.readTouchScenePanelPreference(),null,'unknown stored values fall back to auto');
globalThis.localStorage={getItem:()=>{throw new Error('blocked')},setItem:()=>{throw new Error('blocked')}};
assert.equal(scenes.readTouchScenePanelPreference(),null,'blocked storage falls back to auto');
scenes.writeTouchScenePanelPreference('open');
delete globalThis.localStorage;

const place=(placement,headerOverflows,freeWidth,tempoFootprint)=>nextTopbarTempoPlacement({placement,headerOverflows,freeWidth,tempoFootprint});
assert.equal(place('header',false,0,0),'header','the row keeps BPM/Tap while it fits');
assert.equal(place('header',true,0,135),'tools','only an overflowing row discloses them');
assert.equal(place('tools',false,100,135),'tools','stays disclosed while the free width cannot hold the footprint');
assert.equal(place('tools',false,135,135),'header','returns once the footprint fits, so it cannot oscillate');
assert.equal(place('tools',false,500,0),'tools','an unmeasured footprint never guesses');
console.log('PASS: Scene panel groups every Scene by Bank, fails closed, honours free columns and the stored choice; header tempo placement is stable');
