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
function harness({outputs=[],inputName="SMC-Mixer",feedbackFailure=false,notifyConnection=false,connectionStatus={schema_version:1,project_epoch:3,input_session:41}}={}){let authority={...token},connected=true,mappings=[{message:'ControlChange',number:7,channel:0,action:'LightingMaster',low:0,high:1}],ready,finish,unsubscribed=0,busy=false,feedbackConnected=false,feedbackEnabled=false,selectedOutput=null;const calls=[],messages=[],connections=[];
const options={onMidiControlConnection:notifyConnection?c=>connections.push(c):undefined,invoke:async(command,args)=>{calls.push({command,args});if(command==='list_midi_outputs')return outputs;if(command==='connect_midi_feedback'){if(feedbackFailure)throw new Error('feedback unavailable');feedbackConnected=true;}if(command==='set_midi_feedback_auto')feedbackEnabled=args.enabled;if(command==='disconnect_midi_control')connected=false;if(command==='learn_midi_control')return new Promise(resolve=>{finish=resolve});if(command==='connect_midi_control'){assert.deepEqual(args.mappings,mappings);connected=true;return connectionStatus;}},
flushProjectControlMappingsAuthority:async()=>{authority={...authority,project_revision:authority.project_revision+1,checkpoint_hash:'b'.repeat(64)};return{trusted:true,ownAcknowledgements:[{...authority}]};},
captureProjectAuthorityIdentity:()=>({...authority}),isProjectAuthorityIdentityCurrent:c=>JSON.stringify(c)===JSON.stringify(authority),
midiInputs:()=>[{index:0,name:inputName}],midiOutputs:()=>outputs,setMidiOutputs:()=>{},selectedMidiOutput:()=>selectedOutput,setSelectedMidiOutput:v=>{selectedOutput=v},midiFeedbackConnected:()=>feedbackConnected,setMidiFeedbackConnected:v=>{feedbackConnected=v},midiFeedbackEnabled:()=>feedbackEnabled,setMidiFeedbackEnabled:v=>{feedbackEnabled=v},selectedMidiInput:()=>0,midiControlConnected:()=>connected,setMidiControlConnected:v=>{connected=v},midiMappings:()=>mappings,setMidiMappings:v=>{mappings=typeof v==='function'?v(mappings):v},
setMidiMapChannel:()=>{},setMidiMapMessage:()=>{},setMidiMapNumber:()=>{},setMessage:m=>messages.push(m),onMidiLearnBusy:v=>{busy=v},listenMidiLearnReady:async handler=>{ready=handler;return()=>{unsubscribed++}}};
const controller=createControlInputController(options);
return{controller,calls,messages,connections,get busy(){return busy},get connected(){return connected},get mappings(){return mappings},get unsubscribed(){return unsubscribed},emitReady:payload=>ready(payload),finish:value=>finish(value),replaceProject:()=>{authority={...authority,project_epoch:authority.project_epoch+1}}};}
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

const sceneTarget={action:'TriggerCue',midi_action:'ToggleCue',cue_id:9,label:'Scene'};
for(const [name,outputs,expected,failure] of [
 ['exact output',[{index:7,name:'SMC-Mixer'}],true,false],
 ['different output',[{index:7,name:'SMC-Mixer-bt'}],false,false],
 ['ambiguous output',[{index:7,name:'SMC-Mixer'},{index:8,name:'SMC-Mixer'}],false,false],
 ['no output',[],false,false],
 ['connection failure',[{index:7,name:'SMC-Mixer'}],true,true],
]){const f=harness({outputs,feedbackFailure:failure}),fw=f.controller.learnMidiControlForTargets([sceneTarget]);await waitForLearn(f);f.finish(learned);assert.equal(await fw,true,name+' preserves learned mapping');assert.equal(f.mappings.at(-1).action,'ToggleCue');assert.equal(f.mappings.at(-1).feedback.on.value,127);assert.equal(f.mappings.at(-1).feedback.off.value,0);assert.equal(f.calls.some(c=>c.command==='connect_midi_feedback'),expected,name);assert.equal(f.calls.some(c=>c.command==='set_midi_feedback_auto'),expected&&!failure,name);if(expected&&!failure){const fb=f.calls.find(c=>c.command==='connect_midi_feedback');assert.equal(fb.args.expectedOutputName,'SMC-Mixer');assert.equal(fb.args.outputIndex,7);} }
const flash=harness({outputs:[{index:7,name:'SMC-Mixer'}]}),flashWork=flash.controller.learnMidiControlForTargets([{...sceneTarget,midi_action:'FlashCue'}]);await waitForLearn(flash);flash.finish({...learned,message:'NoteOn'});assert.equal(await flashWork,true);assert.equal(flash.mappings.at(-1).action,'FlashCue');assert.equal(flash.mappings.at(-1).feedback.off.value,0);
console.log('PASS: scene toggle/Flash feedback creation, exact output auto-connect, missing/ambiguous output rejection and nonfatal feedback failure');

const notified=harness({notifyConnection:true});await notified.controller.connectMidiControl();assert.equal(notified.connections.length,1);assert.equal(notified.connections[0].input_session,41);
const invalidIdentity=harness({notifyConnection:true,connectionStatus:{schema_version:2,project_epoch:3,input_session:41}});await invalidIdentity.controller.connectMidiControl();assert.equal(invalidIdentity.connected,false);assert.equal(invalidIdentity.connections.length,0);assert.ok(invalidIdentity.messages.some(m=>m.includes("identity was invalid")));
console.log("PASS: MIDI input event connection identity is published only after valid native ACK");
