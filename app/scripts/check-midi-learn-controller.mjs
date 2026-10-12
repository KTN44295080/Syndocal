import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import ts from 'typescript';
const require=createRequire(import.meta.url),cache=new Map();
const root=resolve(dirname(fileURLToPath(import.meta.url)),'../src');
function load(file){if(cache.has(file))return cache.get(file);const exports={};cache.set(file,exports);
const js=ts.transpileModule(readFileSync(file,'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText;
new Function('require','exports',js)(name=>{if(!name.startsWith('.'))return require(name);const target=resolve(dirname(file),name.endsWith('.ts')?name:name+'.ts');assert.ok(existsSync(target));return load(target);},exports);return exports;}
const {createControlInputController}=load(resolve(root,'createControlInputController.ts'));
const token={project_epoch:3,project_revision:4,checkpoint_hash:'a'.repeat(64)};
const learned={channel:0,message:'ControlChange',number:21,value:100};
function harness(){let authority={...token},connected=true,mappings=[{message:'ControlChange',number:7,channel:0,action:'LightingMaster',low:0,high:1}],ready,finish,unsubscribed=0,busy=false;const calls=[],messages=[];
const options={invoke:async(command,args)=>{calls.push({command,args});if(command==='disconnect_midi_control')connected=false;if(command==='learn_midi_control')return new Promise(resolve=>{finish=resolve});if(command==='connect_midi_control'){assert.deepEqual(args.mappings,mappings);connected=true;}},
flushProjectControlMappingsAuthority:async()=>{authority={...authority,project_revision:authority.project_revision+1,checkpoint_hash:'b'.repeat(64)};return{trusted:true,ownAcknowledgements:[{...authority}]};},
captureProjectAuthorityIdentity:()=>({...authority}),isProjectAuthorityIdentityCurrent:c=>JSON.stringify(c)===JSON.stringify(authority),
selectedMidiInput:()=>0,midiControlConnected:()=>connected,setMidiControlConnected:v=>{connected=v},midiMappings:()=>mappings,setMidiMappings:v=>{mappings=typeof v==='function'?v(mappings):v},
setMidiMapChannel:()=>{},setMidiMapMessage:()=>{},setMidiMapNumber:()=>{},setMessage:m=>messages.push(m),onMidiLearnBusy:v=>{busy=v},listenMidiLearnReady:async handler=>{ready=handler;return()=>{unsubscribed++}}};
const controller=createControlInputController(options);
return{controller,calls,messages,get busy(){return busy},get connected(){return connected},get mappings(){return mappings},get unsubscribed(){return unsubscribed},emitReady:payload=>ready(payload),finish:value=>finish(value),replaceProject:()=>{authority={...authority,project_epoch:authority.project_epoch+1}}};}
const waitForLearn=async h=>{for(let i=0;i<20&&!h.calls.some(c=>c.command==='learn_midi_control');i++)await Promise.resolve();assert.equal(h.calls.at(-1).command,'learn_midi_control');return h.calls.at(-1).args;};
const h=harness(),work=h.controller.learnMidiControlForTargets([{action:'LightingMaster',label:'Master'}]);
const args=await waitForLearn(h);assert.equal(h.calls[0].command,'disconnect_midi_control');assert.equal(h.busy,true);
await h.controller.learnMidiControl();assert.equal(h.calls.filter(c=>c.command==='learn_midi_control').length,1,'single-flight spans Control and Setup');
h.emitReady({learn_request_id:'foreign',project_epoch:3});assert.ok(!h.messages.some(m=>m.includes('input ready')));
h.emitReady({learn_request_id:args.learnRequestId,project_epoch:3});assert.ok(h.messages.some(m=>m.includes('input ready')));
h.finish(learned);assert.equal(await work,true);assert.equal(h.connected,true);assert.equal(h.busy,false);assert.equal(h.unsubscribed,1);assert.equal(h.mappings.at(-1).number,21);assert.equal(h.calls.at(-1).command,'connect_midi_control');
const canceled=harness(),cwork=canceled.controller.learnMidiControlForTargets([{action:'LightingMaster',label:'Master'}]);await waitForLearn(canceled);canceled.replaceProject();canceled.finish(learned);assert.equal(await cwork,false);assert.equal(canceled.mappings.length,1);assert.equal(canceled.calls.some(c=>c.command==='connect_midi_control'),false);assert.equal(canceled.busy,false);assert.equal(canceled.unsubscribed,1);
const setup=harness(),swork=setup.controller.learnMidiControl();await waitForLearn(setup);setup.finish(null);await swork;assert.equal(setup.connected,true,'Setup timeout restores its prior connection');assert.equal(setup.busy,false);assert.equal(setup.unsubscribed,1);
console.log('PASS: real MIDI Learn controller, readiness, cross-screen single-flight, mapping/ACK/reconnect, stale project rejection, Setup timeout restoration');
