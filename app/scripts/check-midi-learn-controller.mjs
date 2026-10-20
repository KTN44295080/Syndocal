import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createRequire } from 'node:module';
import ts from 'typescript';
const require=createRequire(import.meta.url),cache=new Map();
const root=resolve(dirname(fileURLToPath(import.meta.url)),'../src');
function load(file,runtimeRequire=require,moduleCache=cache){if(moduleCache.has(file))return moduleCache.get(file);const exports={};moduleCache.set(file,exports);
const js=ts.transpileModule(readFileSync(file,'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText;
new Function('require','exports',js)(name=>{if(!name.startsWith('.'))return runtimeRequire(name);const target=resolve(dirname(file),name.endsWith('.ts')?name:name+'.ts');assert.ok(existsSync(target));return load(target,runtimeRequire,moduleCache);},exports);return exports;}
const {createControlInputController}=load(resolve(root,'createControlInputController.ts'));
const token={project_epoch:3,project_revision:4,checkpoint_hash:'a'.repeat(64)};
const learned={channel:0,message:'ControlChange',number:21,value:100};
function harness({outputs=[],inputName="SMC-Mixer",feedbackFailure=false,connectFailure=false,notifyConnection=false,connectionStatus={schema_version:1,project_epoch:3,input_session:41}}={}){let authority={...token},connected=true,mappings=[{message:'ControlChange',number:7,channel:0,action:'LightingMaster',low:0,high:1}],ready,finish,unsubscribed=0,busy=false,feedbackConnected=false,feedbackEnabled=false,selectedOutput=null;const calls=[],messages=[],connections=[];
const options={onMidiControlConnection:notifyConnection?c=>connections.push(c):undefined,invoke:async(command,args)=>{calls.push({command,args});if(command==='list_midi_outputs')return outputs;if(command==='connect_midi_feedback'){if(feedbackFailure)throw new Error('feedback unavailable');feedbackConnected=true;}if(command==='set_midi_feedback_auto')feedbackEnabled=args.enabled;if(command==='disconnect_midi_control')connected=false;if(command==='learn_midi_control')return new Promise(resolve=>{finish=resolve});if(command==='connect_midi_control'){if(connectFailure)throw new Error('input busy');assert.deepEqual(args.mappings,mappings);connected=true;return connectionStatus;}},
flushProjectControlMappingsAuthority:async()=>{authority={...authority,project_revision:authority.project_revision+1,checkpoint_hash:'b'.repeat(64)};return{trusted:true,ownAcknowledgements:[{...authority}]};},
captureProjectAuthorityIdentity:()=>({...authority}),isProjectAuthorityIdentityCurrent:c=>JSON.stringify(c)===JSON.stringify(authority),
midiInputs:()=>[{index:0,name:inputName}],midiOutputs:()=>outputs,setMidiOutputs:()=>{},selectedMidiOutput:()=>selectedOutput,setSelectedMidiOutput:v=>{selectedOutput=v},midiFeedbackConnected:()=>feedbackConnected,setMidiFeedbackConnected:v=>{feedbackConnected=v},midiFeedbackEnabled:()=>feedbackEnabled,setMidiFeedbackEnabled:v=>{feedbackEnabled=v},selectedMidiInput:()=>0,midiControlConnected:()=>connected,setMidiControlConnected:v=>{connected=v},midiMappings:()=>mappings,setMidiMappings:v=>{mappings=typeof v==='function'?v(mappings):v},
setMidiMapChannel:()=>{},setMidiMapMessage:()=>{},setMidiMapNumber:()=>{},setMessage:m=>messages.push(m),onMidiLearnBusy:v=>{busy=v},listenMidiLearnReady:async handler=>{ready=handler;return()=>{unsubscribed++}}};
const controller=createControlInputController(options);
return{controller,options,calls,messages,connections,get busy(){return busy},get connected(){return connected},get mappings(){return mappings},get unsubscribed(){return unsubscribed},emitReady:payload=>ready(payload),finish:value=>finish(value),replaceProject:()=>{authority={...authority,project_epoch:authority.project_epoch+1}}};}
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
for (const [name,inputName,outputs,expectedName] of [
 ['Windows second port pairs MIDIIN<n> with MIDIOUT<n>','MIDIIN2 (SMC-Mixer)',[{index:6,name:'SMC-Mixer'},{index:7,name:'MIDIOUT2 (SMC-Mixer)'}],'MIDIOUT2 (SMC-Mixer)'],
 ['exact name still wins over the paired name','MIDIIN2 (SMC-Mixer)',[{index:7,name:'MIDIIN2 (SMC-Mixer)'},{index:8,name:'MIDIOUT2 (SMC-Mixer)'}],'MIDIIN2 (SMC-Mixer)'],
 ['ambiguous paired name is rejected','MIDIIN2 (SMC-Mixer)',[{index:7,name:'MIDIOUT2 (SMC-Mixer)'},{index:8,name:'MIDIOUT2 (SMC-Mixer)'}],null],
 ['other ports of the device are never guessed','MIDIIN2 (SMC-Mixer)',[{index:7,name:'SMC-Mixer'},{index:8,name:'MIDIOUT3 (SMC-Mixer)'}],null],
]){const f=harness({outputs,inputName}),fw=f.controller.learnMidiControlForTargets([sceneTarget]);await waitForLearn(f);f.finish(learned);assert.equal(await fw,true,name);const fb=f.calls.find(c=>c.command==='connect_midi_feedback');assert.equal(fb?.args.expectedOutputName??null,expectedName,name);if(!expectedName)assert.ok(f.messages.at(-1).includes('select a matching MIDI output'),name);}
console.log('PASS: scene toggle/Flash feedback creation, exact or Windows-paired output auto-connect, missing/ambiguous output rejection and nonfatal feedback failure');

// Perform targets: Learn keeps their stable ids, and a beat division is a mapping like any other.
const clipTarget={action:'VideoClipSlotTrigger',label:'Layer 1 clip 2',layer_id:1,clip_slot_id:102};
const stageTarget={action:'VideoIsfStageEnabled',label:'Layer 1 FX 1',layer_id:1,isf_stage_id:900};
const paramTarget={action:'VideoIsfParam',label:'Layer 1 gain',layer_id:1,isf_stage_id:900,attribute:'gain'};
const perform=harness(),performWork=perform.controller.learnMidiControlForTargets([clipTarget]);await waitForLearn(perform);perform.finish({...learned,message:'NoteOn'});assert.equal(await performWork,true);
assert.deepEqual([perform.mappings.at(-1).action,perform.mappings.at(-1).layer_id,perform.mappings.at(-1).clip_slot_id],['VideoClipSlotTrigger',1,102],'Learn stores the stable clip slot id');
assert.equal(perform.mappings.at(-1).feedback,undefined,'Perform targets echo nothing back to the controller');
const beat=harness();assert.equal(beat.connected,true);
assert.equal(await beat.controller.assignBeatDivisionForTargets([clipTarget,stageTarget],4),true);
const beatMappings=beat.mappings.filter(m=>m.message==='BeatDivision');
assert.deepEqual(beatMappings.map(m=>[m.action,m.number,m.channel,m.clip_slot_id??null,m.isf_stage_id??null]),[['VideoClipSlotTrigger',4,null,102,null],['VideoIsfStageEnabled',4,null,null,900]]);
assert.equal(beat.mappings[0].action,'LightingMaster','existing mappings are kept');
assert.equal(beat.calls.filter(c=>c.command==='connect_midi_control').length,1,'a connected worker is re-seated on the new list');
assert.deepEqual(beat.calls.find(c=>c.command==='connect_midi_control').args.mappings,beat.mappings);
assert.equal(beat.connected,true);
assert.ok(beat.messages.some(m=>m.includes('Assigned Layer 1 clip 2 to every 1 bar')));
await beat.controller.assignBeatDivisionForTargets([clipTarget],2);
assert.deepEqual(beat.mappings.filter(m=>m.message==='BeatDivision'&&m.action==='VideoClipSlotTrigger').map(m=>m.number),[2],'one tempo source per target: a new division replaces the old one');
assert.equal(beat.mappings.filter(m=>m.action==='VideoIsfStageEnabled').length,1);
assert.equal(await beat.controller.assignBeatDivisionForTargets([paramTarget],2),false,'a continuous target cannot take a beat');
assert.ok(beat.messages.at(-1).includes('only trigger'));
assert.equal(beat.mappings.some(m=>m.action==='VideoIsfParam'),false);
const removable=beat.mappings.filter(m=>m.message==='BeatDivision');
assert.equal(await beat.controller.removeMidiAssignments(removable,'Layer 1 clip 2'),true);
assert.equal(beat.mappings.some(m=>m.message==='BeatDivision'),false);
assert.deepEqual(beat.mappings.map(m=>m.action),['LightingMaster']);
// A re-seat that fails says why and is not reported as an assignment that worked.
const failing=harness({connectFailure:true});
assert.equal(await failing.controller.assignBeatDivisionForTargets([clipTarget],4),false);
assert.ok(failing.messages.at(-1).includes('input busy'),'the failed re-seat keeps its own error');
assert.ok(!failing.messages.some(m=>m.startsWith('Assigned')));
assert.equal(failing.connected,false);
assert.ok(failing.mappings.some(m=>m.message==='BeatDivision'),'the mapping itself is stored');
// Two assignments made at once re-seat one after the other, each against the list it saw.
const racing=harness();
assert.deepEqual(await Promise.all([racing.controller.assignBeatDivisionForTargets([clipTarget],2),racing.controller.assignBeatDivisionForTargets([stageTarget],3)]),[true,true]);
const seats=racing.calls.filter(c=>c.command==='connect_midi_control');
assert.equal(seats.length,2);
assert.equal(seats[0].args.mappings.filter(m=>m.message==='BeatDivision').length,1,'the first re-seat saw only the first assignment');
assert.equal(seats[1].args.mappings.filter(m=>m.message==='BeatDivision').length,2,'the second saw both');
const offline=harness();await offline.controller.disconnectMidiControl();const before=offline.calls.length;
assert.equal(await offline.controller.assignBeatDivisionForTargets([clipTarget],5),true);
assert.equal(offline.calls.slice(before).some(c=>c.command==='connect_midi_control'),false,'a disconnected input is left disconnected');
assert.equal(offline.mappings.at(-1).number,5);
const sole=harness();await sole.controller.assignBeatDivisionForTargets([clipTarget],3);await sole.controller.removeMidiAssignments(sole.mappings.filter(m=>m.message==='BeatDivision'),'x');
assert.ok(sole.mappings.length>0,'the controller mapping stayed');
const busy=harness(),busyWork=busy.controller.learnMidiControlForTargets([clipTarget]);await waitForLearn(busy);
assert.equal(await busy.controller.assignBeatDivisionForTargets([clipTarget],2),false,'no assignment edit while a Learn is listening');
busy.finish(null);await busyWork;
console.log('PASS: Perform Learn keeps stable ids; beat division assign/replace/remove edit the same mapping list and re-seat a connected input');

const notified=harness({notifyConnection:true});await notified.controller.connectMidiControl();assert.equal(notified.connections.length,1);assert.equal(notified.connections[0].input_session,41);
const invalidIdentity=harness({notifyConnection:true,connectionStatus:{schema_version:2,project_epoch:3,input_session:41}});await invalidIdentity.controller.connectMidiControl();assert.equal(invalidIdentity.connected,false);assert.equal(invalidIdentity.connections.length,0);assert.ok(invalidIdentity.messages.some(m=>m.includes("identity was invalid")));
console.log("PASS: MIDI input event connection identity is published only after valid native ACK");


// OSC Learn uses the same trusted mapping continuation and returned connection identities.
function oscHarness({ deferredLearn = false, learnedResult = { address: "/perform", value: 1, argument_count: 1 } } = {}) {
  let authority = { ...token }, running = true, mappings = [{ address: "/master", action: "LightingMaster", low: 0, high: 1 }], finish;
  const calls = [], messages = [], connections = [];
  const options = {
    invoke: async (command, args) => {
      calls.push({ command, args });
      if (command === "learn_osc_control") {
        if (learnedResult instanceof Error) throw learnedResult;
        return deferredLearn ? new Promise(resolve => { finish = resolve; }) : learnedResult;
      }
      if (command === "start_osc_input") return { schema_version: 1, project_epoch: args.expectedEpoch, input_session: 71 };
    },
    flushProjectControlMappingsAuthority: async () => {
      authority = { ...authority, project_revision: authority.project_revision + 1, checkpoint_hash: "b".repeat(64) };
      return { trusted: true, ownAcknowledgements: [{ ...authority }] };
    },
    captureProjectAuthorityIdentity: () => ({ ...authority }),
    isProjectAuthorityIdentityCurrent: captured => JSON.stringify(captured) === JSON.stringify(authority),
    oscRunning: () => running, setOscRunning: next => { running = next; },
    oscMappings: () => mappings, setOscMappings: next => { mappings = typeof next === "function" ? next(mappings) : next; },
    oscBindIp: () => "127.0.0.1", oscPort: () => 5410,
    setOscMapAddress: () => {}, setMessage: message => messages.push(message),
    onOscControlConnection: connection => connections.push(connection),
  };
  return { controller: createControlInputController(options), calls, messages, connections, get mappings() { return mappings; }, get running() { return running; }, finish: result => finish(result), replaceProject: () => { authority = { ...authority, project_epoch: authority.project_epoch + 1 }; } };
}
const oscSlot = { action: "VideoClipSlotTrigger", label: "Clip", layer_id: 1, clip_slot_id: 102 };
const osc = oscHarness();
assert.equal(await osc.controller.learnOscControlForTargets([oscSlot]), true);
assert.deepEqual(osc.calls.map(call => call.command), ["stop_osc_input", "learn_osc_control", "start_osc_input"]);
assert.equal(osc.mappings.at(-1).clip_slot_id, 102);
assert.equal(osc.connections.length, 1);
assert.equal(osc.running, true);
const oscTimeout = oscHarness({ learnedResult: null });
assert.equal(await oscTimeout.controller.learnOscControlForTargets([oscSlot]), false);
assert.equal(oscTimeout.connections.length, 1);
assert.equal(oscTimeout.mappings.length, 1);
const oscError = oscHarness({ learnedResult: new Error("test OSC failure") });
assert.equal(await oscError.controller.learnOscControlForTargets([oscSlot]), false);
assert.equal(oscError.connections.length, 1);
assert.equal(oscError.running, true);
const oscCanceled = oscHarness({ deferredLearn: true });
const pendingOsc = oscCanceled.controller.learnOscControlForTargets([oscSlot]);
for (let i = 0; i < 20 && !oscCanceled.calls.some(call => call.command === "learn_osc_control"); i++) await Promise.resolve();
assert.ok(oscCanceled.calls.some(call => call.command === "learn_osc_control"));
oscCanceled.replaceProject();
oscCanceled.finish({ address: "/perform", value: 1, argument_count: 1 });
assert.equal(await pendingOsc, false);
assert.equal(oscCanceled.mappings.length, 1);
assert.equal(oscCanceled.connections.length, 0);
assert.equal(oscCanceled.calls.some(call => call.command === "start_osc_input"), false);
const oscStart = oscHarness();
await oscStart.controller.startOscInput();
assert.equal(oscStart.connections.length, 1);
await oscStart.controller.stopOscInput();
assert.equal(oscStart.running, false);
console.log("PASS: OSC Perform Learn, trusted mapping ACK, all listener restoration paths, and stale project cancellation");


// Audio edits must wait for a trusted save ACK and retain the queued project.
const audioSettings = { source: "Level", gain: 1, attack_ms: 20, release_ms: 100,
  threshold: 0.5, mode: "Continuous", low: 0, high: 1 };
const audioTarget = { action: "VideoParam", video_param: "Opacity", layer_id: 1, label: "Layer opacity" };
const audioEdit = harness();
assert.equal(await audioEdit.controller.assignAudioForTargets([audioTarget], audioSettings), true);
const audioId = audioEdit.mappings.find(mapping => mapping.message === "Audio").audio.id;
assert.equal(await audioEdit.controller.assignAudioForTargets([audioTarget], { ...audioSettings, gain: 2 }), true);
assert.equal(audioEdit.mappings.find(mapping => mapping.message === "Audio").audio.id, audioId);
assert.equal(audioEdit.calls.filter(call => call.command === "connect_midi_control").length, 2);
const untrusted = harness();
await untrusted.controller.disconnectMidiControl();
untrusted.options.flushProjectControlMappingsAuthority = async () => ({ trusted: false, ownAcknowledgements: [] });
assert.equal(await untrusted.controller.assignAudioForTargets([audioTarget], audioSettings), false);
assert.equal(untrusted.messages.at(-1), "Control assignments could not be saved.");
assert.equal(untrusted.messages.includes("Audio mapping assigned."), false);
assert.equal(untrusted.calls.some(call => call.command === "connect_midi_control"), false);
const queuedAudio = harness();
const queuedAssignment = queuedAudio.controller.assignAudioForTargets([audioTarget], audioSettings);
queuedAudio.replaceProject();
assert.equal(await queuedAssignment, false);
assert.equal(queuedAudio.mappings.some(mapping => mapping.message === "Audio"), false);
for (const disconnected of [false, true]) {
  const crossing = harness();
  if (disconnected) await crossing.controller.disconnectMidiControl();
  let completeFlush;
  crossing.options.flushProjectControlMappingsAuthority = () => new Promise(resolve => { completeFlush = resolve; });
  const pendingAudio = crossing.controller.assignAudioForTargets([audioTarget], audioSettings);
  for (let i = 0; i < 20 && !completeFlush; i++) await Promise.resolve();
  assert.ok(completeFlush, "the edit reached its persistence barrier");
  crossing.replaceProject();
  completeFlush({ trusted: true, ownAcknowledgements: [] });
  assert.equal(await pendingAudio, false);
  assert.equal(crossing.calls.some(call => call.command === "connect_midi_control"), false,
    "an old Audio assignment never reconnects MIDI against the replacement project");
  assert.ok(crossing.messages.at(-1).includes("Project changed"));
}
const { controlMappingTargetLabel } = load(resolve(root, "controlMappingLabels.ts"));
assert.equal(controlMappingTargetLabel({ ...audioTarget, layer_id: null, group_id: "27" }), "Video Group 27 Opacity");
console.log("PASS: Audio stable assignment identity, trusted ACK, queued/in-flight project replacement, MIDI reseat fencing and group target label");


// Removal shares the same durable, project-fenced queue as assignment.
const removing = harness();
await removing.controller.assignAudioForTargets([audioTarget], audioSettings);
const oldAudio = removing.mappings.filter(mapping => mapping.message === "Audio");
const removeWork = removing.controller.removeMidiAssignments(oldAudio, "Layer opacity");
const connectsBefore = removing.calls.filter(call => call.command === "connect_midi_control").length;
removing.replaceProject();
assert.equal(await removeWork, false);
assert.equal(removing.calls.filter(call => call.command === "connect_midi_control").length, connectsBefore);
const removeUntrusted = harness();
await removeUntrusted.controller.disconnectMidiControl();
removeUntrusted.options.flushProjectControlMappingsAuthority = async () => ({ trusted: false, ownAcknowledgements: [] });
assert.equal(await removeUntrusted.controller.removeMidiAssignments(removeUntrusted.mappings, "Master"), false);
assert.equal(removeUntrusted.messages.at(-1), "Control assignments could not be saved.");


// Exercise actual Solid effects; Node's default Solid server build skips them.
const solid = require("solid-js/dist/solid.cjs");
const reactiveController = load(resolve(root, "createControlInputController.ts"),
  name => name === "solid-js" ? solid : require(name), new Map()).createControlInputController;
const until = async predicate => {
  for (let i = 0; i < 80 && !predicate(); i++) await Promise.resolve();
  assert.ok(predicate(), "reactive operation reached the expected boundary");
};
function reactiveFeedbackHarness() {
  const base = harness(), calls = [], messages = [], flushes = [];
  let authority = { ...token }, dispose;
  const [mappings, setMappings] = solid.createSignal(base.mappings);
  const [connected, setConnected] = solid.createSignal(true);
  const [enabled, setEnabled] = solid.createSignal(true);
  solid.createRoot(cleanup => {
    dispose = cleanup;
    reactiveController({ ...base.options,
      midiMappings: mappings, setMidiMappings: setMappings,
      midiFeedbackConnected: connected, setMidiFeedbackConnected: setConnected,
      midiFeedbackEnabled: enabled, setMidiFeedbackEnabled: setEnabled,
      captureProjectAuthorityIdentity: () => ({ ...authority }),
      isProjectAuthorityIdentityCurrent: captured => JSON.stringify(captured) === JSON.stringify(authority),
      flushProjectControlMappingsAuthority: () => new Promise(resolve => { flushes.push(resolve); }),
      invoke: async (command, args) => { calls.push({ command, args }); return { enabled: args?.enabled }; },
      setMessage: message => messages.push(message),
    });
  });
  return { calls, messages, flushes, mappings, setMappings, setEnabled, dispose,
    replaceProject: () => { authority = { ...authority, project_epoch: authority.project_epoch + 1 }; },
    complete(trusted = true) {
      const finish = flushes.shift();
      assert.ok(finish, "a flush is pending");
      if (trusted) authority = { ...authority, project_revision: authority.project_revision + 1, checkpoint_hash: "b".repeat(64) };
      finish({ trusted, ownAcknowledgements: trusted ? [{ ...authority }] : [] });
    },
  };
}
const savedWindow = globalThis.window;
globalThis.window = { setInterval: () => 1, clearInterval: () => {} };
try {
  const feedback = reactiveFeedbackHarness();
  await until(() => feedback.flushes.length === 1);
  assert.equal(feedback.calls.length, 0, "feedback waits for the initial persistence ACK");
  feedback.complete();
  await until(() => feedback.calls.length === 1);
  const audioMappings = audioEdit.mappings;
  feedback.setMappings(audioMappings);
  await until(() => feedback.flushes.length === 1);
  assert.equal(feedback.calls.length, 1, "Audio edits cannot race ahead of persistence");
  feedback.complete();
  await until(() => feedback.calls.length === 2);
  assert.equal(feedback.calls.at(-1).command, "set_midi_feedback_auto");
  assert.deepEqual(feedback.calls.at(-1).args.mappings, audioMappings);
  feedback.setMappings([...audioMappings]);
  await until(() => feedback.flushes.length === 1);
  feedback.setEnabled(false);
  feedback.complete();
  await until(() => feedback.flushes.length === 1);
  assert.equal(feedback.calls.length, 2, "superseded feedback configuration is discarded");
  feedback.complete();
  await until(() => feedback.calls.length === 3);
  assert.equal(feedback.calls.at(-1).args.enabled, false);
  feedback.setMappings([...audioMappings]);
  await until(() => feedback.flushes.length === 1);
  feedback.replaceProject();
  feedback.complete();
  for (let i = 0; i < 20; i++) await Promise.resolve();
  assert.equal(feedback.calls.length, 3, "pending feedback cannot cross project replacement");
  feedback.dispose();

  const rejectedFeedback = reactiveFeedbackHarness();
  await until(() => rejectedFeedback.flushes.length === 1);
  rejectedFeedback.complete(false);
  await until(() => rejectedFeedback.messages.length === 1);
  assert.equal(rejectedFeedback.calls.length, 0, "untrusted persistence never configures feedback");
  assert.equal(rejectedFeedback.messages[0], "Control assignments could not be saved.");
  rejectedFeedback.dispose();
} finally { globalThis.window = savedWindow; }
console.log("PASS: reactive MIDI feedback waits for Audio mapping ACK, uses current mappings, and rejects superseded/project-crossing/untrusted continuations");
